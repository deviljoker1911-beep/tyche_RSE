# Tyche datasets

Public, free, no-auth datasets useful for **calibrating** the Tyche
simulation core and **stress-testing** the attestation pipeline against
real-shaped inputs.

> **Important**: nothing in here is a substitute for actual private-credit
> portfolio data. The simulator is designed so that production firms run it
> on *their own* portfolios — Tyche never sees them. The datasets here are
> for tuning the structural model parameters and producing realistic
> synthetic books for backtesting and demos.

## Layout

```
data/
├── raw/                 # As-downloaded files (gzipped, zipped, JSON, CSV).
│   ├── ecb/             # Euro-area credit, lending, rates
│   ├── fred/            # US/global macro time series
│   ├── eurostat/        # GDP, GVA, HICP, industrial production, bankruptcy
│   ├── bis/             # Credit-to-non-financial-sector, DSR, banking
│   ├── worldbank/       # Country-level macro / financial indicators
│   ├── oecd/            # National accounts, key indicators
│   ├── boe/             # Bank of England UK lending series
│   ├── eba/             # EBA Risk Dashboard XLSX
│   ├── sec_edgar/       # Quarterly Financial Statement Data Sets
│   ├── loan_level/      # Lending Club / Freddie Mac samples, CFPB complaints
│   └── credit_pdfs/     # Moody's/S&P annual default studies, IMF GFSR, ECB FSR
├── processed/           # Unzipped, format-normalised outputs
├── scripts/
│   ├── fetch_all.sh     # The downloader. Idempotent; pass --force to refetch.
│   └── postprocess.sh   # Unzips and generates MANIFEST.md
└── MANIFEST.md          # Generated catalogue of what's actually present.
```

## Usage

```sh
bash data/scripts/fetch_all.sh
bash data/scripts/postprocess.sh
ls data/processed/
```

After both scripts complete, `data/MANIFEST.md` lists every file with its
size. Some endpoints will fail intermittently (sites change layout); rerun
the fetcher to retry.

## What's *not* in here, and why

| Source | Why it's not auto-downloaded |
|---|---|
| FRED API (full series) | Free but needs an API key. Get one at https://fred.stlouisfed.org/docs/api/api_key.html and set `FRED_API_KEY=...` |
| UK Companies House bulk financial data | Free but needs an API key. https://developer.company-information.service.gov.uk/ |
| Kaggle (Lending Club full, Home Credit, etc.) | Needs a Kaggle account + API token. https://www.kaggle.com/docs/api |
| WRDS / DealScan / Compustat | Behind academic paywall |
| Preqin / Pitchbook / Cliffwater constituents | Commercial paywall |
| Moody's CreditEdge / S&P CreditPro | Commercial paywall |
| ESMA AIFMD raw files | Some require ESMA registry registration |
| Bloomberg DRSK | Commercial |

The fetch script ignores rate limits politely (`--retry 3`, single-threaded
per-host). If you re-run it weekly, you'll stay current.

## Calibration paths into Tyche

| Dataset | Maps to | How |
|---|---|---|
| ECB MFI lending rates / volumes | `Loan.coupon`, `Loan.principal` distribution | Calibrate the `generate.py` synthetic book against the empirical distribution of EU NFC lending |
| EBA Risk Dashboard NPL by country & sector | `Loan.leverage` → PD anchor | Re-fit the `1.5` constant in `derive_pd_curve` so simulator PDs match published NPL ratios |
| BIS credit-to-non-financial-sector | macro `gdp_shock` calibration | Build historical scenario set: when GDP fell X%, NFC-credit fell Y% in this country |
| Eurostat sectoral GVA | `Sector` weights in synthetic book | Replace the uniform sector mix with the empirical EU NFC GVA distribution |
| FRED `BAMLH0A0HYM2` (HY OAS) | `MacroScenario.rate_shock_bps` | Shock magnitude under stress is the historical 99% percentile of HY-OAS widening |
| Eurostat `bs_bs7_q` (bankruptcies) | sanity check on EL output | Total simulated EL across the EU should match observed bankruptcy-driven losses within order of magnitude |
| Moody's annual default study | benchmark for sim outputs | EL by sector should be in the ballpark of Moody's published 1y default rates × LGD |

A worked example lives in [`data/calibration.ipynb`](calibration.ipynb)
(stub — extend in your own branch).
