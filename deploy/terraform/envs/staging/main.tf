# Staging env composition: glues the modules together for eu-west-1.
# Production envs are this file with smaller numbers turned up.

terraform {
  backend "s3" {
    bucket         = "tyche-tfstate-mgmt"
    key            = "tyche/staging/terraform.tfstate"
    region         = "eu-west-1"
    dynamodb_table = "tyche-tfstate-lock"
    encrypt        = true
    role_arn       = "arn:aws:iam::STAGING_ACCOUNT_ID:role/OrganizationAccountAccessRole"
  }
}

provider "aws" {
  region = "eu-west-1"
  assume_role {
    role_arn = "arn:aws:iam::STAGING_ACCOUNT_ID:role/OrganizationAccountAccessRole"
  }
  default_tags {
    tags = {
      Project     = "tyche"
      Environment = "staging"
      ManagedBy   = "terraform"
      CostCenter  = "engineering"
    }
  }
}

locals {
  name = "tyche-staging"
  tags = {
    Project     = "tyche"
    Environment = "staging"
  }
}

module "network" {
  source = "../../modules/network"
  name   = local.name
  cidr   = "10.10.0.0/16"
  azs    = ["eu-west-1a", "eu-west-1b", "eu-west-1c"]
  tags   = local.tags
}

module "eks" {
  source              = "../../modules/eks"
  name                = local.name
  kubernetes_version  = "1.31"
  vpc_id              = module.network.vpc_id
  private_subnet_ids  = module.network.private_subnet_ids
  public_subnet_ids   = module.network.public_subnet_ids
  node_instance_types = ["m7g.large"]
  node_desired_size   = 3
  node_max_size       = 6
  tags                = local.tags
}

module "data" {
  source              = "../../modules/data"
  name                = local.name
  vpc_id              = module.network.vpc_id
  vpc_cidr            = module.network.vpc_cidr
  database_subnet_ids = module.network.database_subnet_ids
  tags                = local.tags
}

module "security" {
  source = "../../modules/security"
  name   = local.name
  tags   = local.tags
}

output "kubeconfig_command" {
  value = "aws eks update-kubeconfig --name ${module.eks.cluster_name} --region eu-west-1"
}
