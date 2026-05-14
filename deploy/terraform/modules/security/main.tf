# Per-account security baseline:
#   - CloudTrail to the shared `security` account audit bucket
#   - GuardDuty enabled (delegated admin = security account)
#   - Security Hub enabled (CIS + AWS Foundational best practices)
#   - AWS Config recording all supported resources
#   - IAM password policy + access analyzer

variable "name"             { type = string }
variable "audit_bucket_arn" { type = string;  default = "" }
variable "tags"             { type = map(string); default = {} }

data "aws_caller_identity" "current" {}
data "aws_region" "current" {}

# ─── CloudTrail ──────────────────────────────────────────────────────────
resource "aws_kms_key" "ct" {
  description             = "${var.name} CloudTrail encryption"
  enable_key_rotation     = true
  deletion_window_in_days = 30
  tags                    = var.tags
}

resource "aws_cloudtrail" "main" {
  name                          = "${var.name}-cloudtrail"
  s3_bucket_name                = var.audit_bucket_arn != "" ? element(split(":", var.audit_bucket_arn), 5) : aws_s3_bucket.ct[0].id
  include_global_service_events = true
  is_multi_region_trail         = true
  enable_log_file_validation    = true
  kms_key_id                    = aws_kms_key.ct.arn
  tags                          = var.tags
}

# Fallback bucket if no external audit bucket given.
resource "aws_s3_bucket" "ct" {
  count  = var.audit_bucket_arn == "" ? 1 : 0
  bucket = "tyche-${var.name}-cloudtrail"
  tags   = var.tags
}

resource "aws_s3_bucket_versioning" "ct" {
  count  = var.audit_bucket_arn == "" ? 1 : 0
  bucket = aws_s3_bucket.ct[0].id
  versioning_configuration { status = "Enabled" }
}

resource "aws_s3_bucket_public_access_block" "ct" {
  count  = var.audit_bucket_arn == "" ? 1 : 0
  bucket = aws_s3_bucket.ct[0].id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_policy" "ct" {
  count  = var.audit_bucket_arn == "" ? 1 : 0
  bucket = aws_s3_bucket.ct[0].id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "AWSCloudTrailAclCheck"
        Effect    = "Allow"
        Principal = { Service = "cloudtrail.amazonaws.com" }
        Action    = "s3:GetBucketAcl"
        Resource  = aws_s3_bucket.ct[0].arn
      },
      {
        Sid       = "AWSCloudTrailWrite"
        Effect    = "Allow"
        Principal = { Service = "cloudtrail.amazonaws.com" }
        Action    = "s3:PutObject"
        Resource  = "${aws_s3_bucket.ct[0].arn}/AWSLogs/${data.aws_caller_identity.current.account_id}/*"
        Condition = {
          StringEquals = { "s3:x-amz-acl" = "bucket-owner-full-control" }
        }
      },
    ]
  })
}

# ─── GuardDuty ───────────────────────────────────────────────────────────
resource "aws_guardduty_detector" "main" {
  enable                       = true
  finding_publishing_frequency = "FIFTEEN_MINUTES"
  datasources {
    s3_logs { enable = true }
    kubernetes { audit_logs { enable = true } }
    malware_protection {
      scan_ec2_instance_with_findings { ebs_volumes { enable = true } }
    }
  }
  tags = var.tags
}

# ─── Security Hub ────────────────────────────────────────────────────────
resource "aws_securityhub_account" "main" {
  enable_default_standards = false
}

resource "aws_securityhub_standards_subscription" "cis" {
  depends_on    = [aws_securityhub_account.main]
  standards_arn = "arn:aws:securityhub:::ruleset/cis-aws-foundations-benchmark/v/1.4.0"
}

resource "aws_securityhub_standards_subscription" "aws_foundational" {
  depends_on    = [aws_securityhub_account.main]
  standards_arn = "arn:aws:securityhub:${data.aws_region.current.name}::standards/aws-foundational-security-best-practices/v/1.0.0"
}

# ─── AWS Config ──────────────────────────────────────────────────────────
resource "aws_config_configuration_recorder" "main" {
  name     = "${var.name}-config"
  role_arn = aws_iam_role.config.arn
  recording_group {
    all_supported                 = true
    include_global_resource_types = true
  }
}

resource "aws_config_delivery_channel" "main" {
  name           = "${var.name}-config"
  s3_bucket_name = var.audit_bucket_arn != "" ? element(split(":", var.audit_bucket_arn), 5) : aws_s3_bucket.ct[0].id
  depends_on     = [aws_config_configuration_recorder.main]
}

resource "aws_config_configuration_recorder_status" "main" {
  name       = aws_config_configuration_recorder.main.name
  is_enabled = true
  depends_on = [aws_config_delivery_channel.main]
}

resource "aws_iam_role" "config" {
  name = "${var.name}-config"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = { Service = "config.amazonaws.com" }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "config" {
  role       = aws_iam_role.config.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWS_ConfigRole"
}

# ─── IAM baseline ────────────────────────────────────────────────────────
resource "aws_iam_account_password_policy" "main" {
  minimum_password_length        = 14
  require_lowercase_characters   = true
  require_uppercase_characters   = true
  require_numbers                = true
  require_symbols                = true
  allow_users_to_change_password = true
  password_reuse_prevention      = 24
  max_password_age               = 90
}

resource "aws_accessanalyzer_analyzer" "main" {
  analyzer_name = "${var.name}-access-analyzer"
  type          = "ORGANIZATION"
}

output "guardduty_detector_id" { value = aws_guardduty_detector.main.id }
output "cloudtrail_arn"        { value = aws_cloudtrail.main.arn }
