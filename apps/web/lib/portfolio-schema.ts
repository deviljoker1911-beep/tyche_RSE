import { z } from "zod";

export const SeniorityZ = z.enum([
  "senior_secured",
  "second_lien",
  "senior_unsecured",
  "subordinated",
  "mezzanine",
  "equity",
]);

export const SectorZ = z.enum([
  "technology",
  "healthcare",
  "industrials",
  "consumer",
  "financials",
  "energy",
  "real_estate",
  "materials",
  "telecom",
  "utilities",
  "other",
]);

export const GeographyZ = z.enum([
  "uk",
  "eu_core",
  "eu_periphery",
  "nordics",
  "switzerland",
  "em_europe",
  "other",
]);

export const CovenantZ = z.object({
  name: z.string(),
  threshold: z.number(),
  direction: z.enum(["leq_threshold", "geq_threshold"]),
  cushion: z.number(),
});

export const LoanZ = z.object({
  loan_id: z.string(),
  issuer: z.string(),
  sector: SectorZ,
  geography: GeographyZ,
  seniority: SeniorityZ,
  principal: z.number().positive(),
  coupon: z.number().min(0).max(1),
  maturity_years: z.number().positive(),
  leverage: z.number().min(0),
  asset_volatility: z.number().positive(),
  covenants: z.array(CovenantZ).default([]),
  collateral_coverage: z.number().min(0),
});

export const PortfolioZ = z.object({
  firm_id: z.string(),
  as_of: z.string(),
  loans: z.array(LoanZ),
});

export const SectorShockZ = z.object({
  ebitda_shock: z.number(),
  asset_shock: z.number(),
});

export const MacroScenarioZ = z.object({
  scenario_id: z.string(),
  name: z.string(),
  gdp_shock: z.number(),
  rate_shock_bps: z.number(),
  sector_shocks: z.record(SectorZ, SectorShockZ).default({}),
});

export const RiskMetricsZ = z.object({
  expected_loss: z.number(),
  var_95: z.number(),
  var_99: z.number(),
  expected_shortfall_975: z.number(),
  sector_contributions: z.record(
    SectorZ,
    z.object({
      expected_loss: z.number(),
      share: z.number(),
    }),
  ),
  n_paths: z.number().int().nonnegative(),
});

export type Portfolio = z.infer<typeof PortfolioZ>;
export type Loan = z.infer<typeof LoanZ>;
export type MacroScenario = z.infer<typeof MacroScenarioZ>;
export type RiskMetrics = z.infer<typeof RiskMetricsZ>;
