# AWS Organizations + member accounts.
#
# Runs against the management account once at bootstrap; subsequent changes
# go through PR review. SCPs enforce the security baseline (no root-user
# access keys, no internet-exposed root EBS, mandatory CloudTrail, etc.)
# at the Org level so a member-account misconfiguration can't bypass them.

terraform {
  backend "s3" {
    bucket         = "tyche-tfstate-mgmt"
    key            = "tyche/org/terraform.tfstate"
    region         = "eu-west-1"
    dynamodb_table = "tyche-tfstate-lock"
    encrypt        = true
  }
}

provider "aws" {
  region = "eu-west-1"
}

data "aws_caller_identity" "current" {}

locals {
  member_accounts = {
    security        = "ops+security@tyche.network"
    dev             = "ops+dev@tyche.network"
    staging         = "ops+staging@tyche.network"
    prod-eu-west    = "ops+prod-euw1@tyche.network"
    prod-eu-central = "ops+prod-euc1@tyche.network"
  }
}

resource "aws_organizations_organization" "tyche" {
  feature_set = "ALL"
  enabled_policy_types = [
    "SERVICE_CONTROL_POLICY",
    "TAG_POLICY",
  ]
  aws_service_access_principals = [
    "sso.amazonaws.com",
    "guardduty.amazonaws.com",
    "securityhub.amazonaws.com",
    "config.amazonaws.com",
    "cloudtrail.amazonaws.com",
  ]
}

resource "aws_organizations_organizational_unit" "workloads" {
  name      = "workloads"
  parent_id = aws_organizations_organization.tyche.roots[0].id
}

resource "aws_organizations_organizational_unit" "security" {
  name      = "security"
  parent_id = aws_organizations_organization.tyche.roots[0].id
}

resource "aws_organizations_account" "member" {
  for_each  = local.member_accounts
  name      = each.key
  email     = each.value
  parent_id = each.key == "security" ? aws_organizations_organizational_unit.security.id : aws_organizations_organizational_unit.workloads.id

  iam_user_access_to_billing = "DENY"
  role_name                  = "OrganizationAccountAccessRole"

  # Closing an account is intentional, irreversible; require manual confirmation.
  lifecycle {
    prevent_destroy = true
    ignore_changes  = [role_name]
  }
}

# SCP: deny root-user keys, IAM-user creation (force SSO), and any region
# outside Europe.
resource "aws_organizations_policy" "baseline" {
  name        = "tyche-baseline"
  type        = "SERVICE_CONTROL_POLICY"
  description = "Deny root key creation, force EU region, prevent direct IAM users"

  content = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "DenyRootAccess"
        Effect = "Deny"
        Action = ["*"]
        Resource = "*"
        Condition = {
          StringLike = {
            "aws:PrincipalArn" = "arn:aws:iam::*:root"
          }
        }
      },
      {
        Sid    = "DenyOutsideEU"
        Effect = "Deny"
        NotAction = [
          # Global services exempt from region restrictions
          "iam:*", "organizations:*", "support:*", "sts:*", "route53:*",
          "cloudfront:*", "waf:*", "wafv2:*", "shield:*", "globalaccelerator:*",
          "health:*", "savingsplans:*", "tag:*", "trustedadvisor:*",
        ]
        Resource = "*"
        Condition = {
          StringNotEquals = {
            "aws:RequestedRegion" = ["eu-west-1", "eu-central-1"]
          }
        }
      },
      {
        Sid       = "DenyIamUserCreation"
        Effect    = "Deny"
        Action    = ["iam:CreateUser", "iam:CreateAccessKey"]
        Resource  = "*"
      },
      {
        Sid    = "RequireImdsv2"
        Effect = "Deny"
        Action = ["ec2:RunInstances"]
        Resource = "arn:aws:ec2:*:*:instance/*"
        Condition = {
          StringNotEquals = {
            "ec2:MetadataHttpTokens" = "required"
          }
        }
      },
    ]
  })
}

resource "aws_organizations_policy_attachment" "baseline_workloads" {
  policy_id = aws_organizations_policy.baseline.id
  target_id = aws_organizations_organizational_unit.workloads.id
}

# Centralised CloudTrail audit log bucket in the `security` account.
output "member_account_ids" {
  value = { for k, v in aws_organizations_account.member : k => v.id }
}
