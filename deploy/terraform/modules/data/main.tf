# Data plane: PostgreSQL (RDS Aurora), Redis (ElastiCache), S3 Object Lock
# for attestation record blobs. ClickHouse lives in ClickHouse Cloud (managed
# by the ClickHouse team) and is wired in by Helm value, not Terraform.

variable "name"               { type = string }
variable "vpc_id"             { type = string }
variable "vpc_cidr"           { type = string }
variable "database_subnet_ids" { type = list(string) }
variable "tags"               { type = map(string); default = {} }

# KMS key shared across data stores in this env.
resource "aws_kms_key" "data" {
  description             = "${var.name} data envelope"
  deletion_window_in_days = 30
  enable_key_rotation     = true
  tags                    = var.tags
}

resource "aws_kms_alias" "data" {
  name          = "alias/${var.name}-data"
  target_key_id = aws_kms_key.data.key_id
}

# ─── PostgreSQL (Aurora) ───────────────────────────────────────────────────
resource "aws_db_subnet_group" "main" {
  name       = "${var.name}-db"
  subnet_ids = var.database_subnet_ids
  tags       = var.tags
}

resource "aws_security_group" "rds" {
  name        = "${var.name}-rds"
  description = "RDS Aurora"
  vpc_id      = var.vpc_id
  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }
  tags = merge(var.tags, { Name = "${var.name}-rds" })
}

resource "random_password" "rds_master" {
  length  = 32
  special = false
}

resource "aws_rds_cluster" "main" {
  cluster_identifier      = "${var.name}-aurora"
  engine                  = "aurora-postgresql"
  engine_version          = "16.4"
  database_name           = "tyche"
  master_username         = "tyche_admin"
  master_password         = random_password.rds_master.result
  backup_retention_period = 14
  preferred_backup_window = "02:00-04:00"
  storage_encrypted       = true
  kms_key_id              = aws_kms_key.data.arn
  vpc_security_group_ids  = [aws_security_group.rds.id]
  db_subnet_group_name    = aws_db_subnet_group.main.name
  copy_tags_to_snapshot   = true
  deletion_protection     = true

  serverlessv2_scaling_configuration {
    min_capacity = 0.5
    max_capacity = 16
  }

  enabled_cloudwatch_logs_exports = ["postgresql"]
  iam_database_authentication_enabled = true
  tags = var.tags
}

resource "aws_rds_cluster_instance" "main" {
  count               = 2  # one writer, one reader
  identifier          = "${var.name}-aurora-${count.index}"
  cluster_identifier  = aws_rds_cluster.main.id
  instance_class      = "db.serverless"
  engine              = aws_rds_cluster.main.engine
  engine_version      = aws_rds_cluster.main.engine_version
  publicly_accessible = false
  tags                = var.tags
}

resource "aws_secretsmanager_secret" "rds_password" {
  name = "tyche/${var.name}/rds-master-password"
  kms_key_id = aws_kms_key.data.arn
  tags = var.tags
}

resource "aws_secretsmanager_secret_version" "rds_password" {
  secret_id = aws_secretsmanager_secret.rds_password.id
  secret_string = jsonencode({
    url      = "postgres://tyche_admin:${random_password.rds_master.result}@${aws_rds_cluster.main.endpoint}:5432/tyche"
    username = "tyche_admin"
    password = random_password.rds_master.result
    host     = aws_rds_cluster.main.endpoint
  })
}

# ─── Redis (ElastiCache) ──────────────────────────────────────────────────
resource "aws_elasticache_subnet_group" "main" {
  name       = "${var.name}-redis"
  subnet_ids = var.database_subnet_ids
}

resource "aws_security_group" "redis" {
  name        = "${var.name}-redis"
  description = "ElastiCache Redis"
  vpc_id      = var.vpc_id
  ingress {
    from_port   = 6379
    to_port     = 6379
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }
}

resource "aws_elasticache_replication_group" "main" {
  replication_group_id       = "${var.name}-redis"
  description                = "Tyche cache + rate limit + idempotency"
  engine                     = "redis"
  engine_version             = "7.1"
  node_type                  = "cache.t4g.small"
  num_cache_clusters         = 2
  parameter_group_name       = "default.redis7"
  port                       = 6379
  subnet_group_name          = aws_elasticache_subnet_group.main.name
  security_group_ids         = [aws_security_group.redis.id]
  automatic_failover_enabled = true
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  kms_key_id                 = aws_kms_key.data.arn
  snapshot_retention_limit   = 7
  tags                       = var.tags
}

# ─── S3 Object Lock for attestation records ──────────────────────────────
resource "aws_s3_bucket" "attestations" {
  bucket = "tyche-${var.name}-attestations"
  object_lock_enabled = true # Required at create time, cannot be changed.
  tags = merge(var.tags, { Sensitivity = "high" })
}

resource "aws_s3_bucket_object_lock_configuration" "attestations" {
  bucket = aws_s3_bucket.attestations.id
  rule {
    default_retention {
      mode = "COMPLIANCE"  # Cannot be overridden, not even by root.
      years = 10
    }
  }
}

resource "aws_s3_bucket_versioning" "attestations" {
  bucket = aws_s3_bucket.attestations.id
  versioning_configuration { status = "Enabled" }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "attestations" {
  bucket = aws_s3_bucket.attestations.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = aws_kms_key.data.arn
    }
    bucket_key_enabled = true
  }
}

resource "aws_s3_bucket_public_access_block" "attestations" {
  bucket                  = aws_s3_bucket.attestations.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "attestations" {
  bucket = aws_s3_bucket.attestations.id
  rule {
    id     = "transition-glacier"
    status = "Enabled"
    filter {}
    transition {
      days          = 90
      storage_class = "GLACIER_IR"
    }
  }
}

output "rds_endpoint"       { value = aws_rds_cluster.main.endpoint }
output "rds_password_arn"   { value = aws_secretsmanager_secret.rds_password.arn }
output "redis_endpoint"     { value = aws_elasticache_replication_group.main.primary_endpoint_address }
output "attestation_bucket" { value = aws_s3_bucket.attestations.id }
output "kms_key_arn"        { value = aws_kms_key.data.arn }
