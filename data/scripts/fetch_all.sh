#!/usr/bin/env bash
# data/scripts/fetch_all.sh
# Pulls publicly downloadable, no-auth datasets useful for calibrating Tyche.
#
# All paths relative to repo root. Idempotent — already-downloaded files are
# skipped unless --force.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RAW="$ROOT/data/raw"
FORCE="${1:-}"

UA="Mozilla/5.0 (compatible; tyche-data-fetcher/0.1; +https://github.com/deviljoker1911-beep/tyche_RSE)"

fetch() {
  local url="$1" out="$2"
  if [[ -f "$out" && "$FORCE" != "--force" ]]; then
    echo "  skip (exists): $(basename "$out")"
    return 0
  fi
  mkdir -p "$(dirname "$out")"
  if curl -fsSL --retry 3 --retry-delay 2 -A "$UA" -o "$out.tmp" "$url"; then
    mv "$out.tmp" "$out"
    local size; size=$(stat -f%z "$out" 2>/dev/null || stat -c%s "$out")
    echo "  ok   ($(numfmt --to=iec "$size" 2>/dev/null || echo "$size B")): $(basename "$out")"
  else
    rm -f "$out.tmp"
    echo "  FAIL: $(basename "$out") — $url" >&2
    return 1
  fi
}

echo "=== FRED (St Louis Fed) — macro time series ==="
declare -a FRED=(
  GDPC1            # US Real GDP
  CPIAUCSL         # US CPI
  DGS10            # 10y Treasury yield
  DFF              # Fed funds rate
  BAMLH0A0HYM2     # ICE BofA US High Yield Index OAS
  BAMLC0A0CM       # ICE BofA US Corp Index OAS
  TEDRATE          # TED spread (discontinued but useful history)
  T10Y3M           # Term spread
  DRBLACBS         # Charge-off rate, all banks
  DRSFRMACBS       # Single family delinquency
  USREC            # NBER recession indicator
  UNRATE           # US unemployment
  HYIOAS           # HY index OAS
  ECBESTRVOLWGTTRMDMNRT  # €STR
  IRSTCB01EZQ156N  # Euro area policy rate
  CLVMNACSCAB1GQEA19   # Euro Area GDP
  CP0000EZ19M086NEST   # Euro area HICP
)
for s in "${FRED[@]}"; do
  fetch "https://fred.stlouisfed.org/graph/fredgraph.csv?id=$s" "$RAW/fred/${s}.csv" || true
done

echo
echo "=== ECB Statistical Data Warehouse ==="
# Format reference: https://data.ecb.europa.eu/help/api/data
# Series naming: dataset/key
declare -a ECB=(
  "BSI/M.U2.N.A.A20.A.1.U2.2240.Z01.E:euro_area_credit_to_NFCs_outstanding"
  "BSI/M.U2.N.A.A20.A.4.U2.2240.Z01.E:euro_area_NFC_loan_growth"
  "BLS/Q.U2.ALL.O.C.S.B3.B5.NF.M.WE:lending_survey_credit_standards_NFC"
  "MIR/M.U2.B.A2A.A.R.A.2240.EUR.N:lending_rates_NFC_new_business"
  "FM/D.U2.EUR.4F.KR.MRR_FR.LEV:euro_area_main_refi_rate"
  "RAI/M.U2.HU.0000.0000.MA.IN.R30:retail_arrears_index"
  "QSA/Q.N.I8.W0.S11.S1.N.L.LE.F4.T._Z.XDC._T.S.V.N._T:NFC_debt_outstanding"
)
for entry in "${ECB[@]}"; do
  key="${entry%%:*}"
  name="${entry##*:}"
  fetch "https://data-api.ecb.europa.eu/service/data/${key}?format=csvdata" "$RAW/ecb/${name}.csv" || true
done

echo
echo "=== Eurostat (full datasets, gzipped CSV) ==="
declare -a EUROSTAT=(
  nama_10_gdp                 # GDP main aggregates
  nama_10_a64                 # GVA by sector NACE (A64)
  prc_hicp_manr               # HICP annual rates
  ei_isen_m                   # Industrial production
  bs_bs7_q                    # Bankruptcy declarations
  ert_bil_eur_a               # FX rates
  irt_lt_mcby_d               # 10y bond yields
  sbs_sc_ovw                  # Structural business statistics
  bd_size_r3                  # Business demography by sector
  fbs_pension_q               # Pension/insurance balance sheets
)
for ds in "${EUROSTAT[@]}"; do
  fetch "https://ec.europa.eu/eurostat/api/dissemination/sdmx/2.1/data/${ds}?format=SDMX-CSV&compressed=true" \
        "$RAW/eurostat/${ds}.csv.gz" || true
done

echo
echo "=== World Bank Open Data ==="
declare -a WB_INDICATORS=(
  NY.GDP.MKTP.CD               # GDP USD
  FR.INR.RINR                  # Real interest rate
  FS.AST.PRVT.GD.ZS            # Domestic credit to private sector / GDP
  GFDD.SI.02                   # Bank NPL ratio
  GFDD.SI.03                   # Bank Z-score
  FS.AST.DOMS.GD.ZS            # Domestic credit / GDP
  CM.MKT.TRAD.GD.ZS            # Stocks traded / GDP
)
for ind in "${WB_INDICATORS[@]}"; do
  fetch "https://api.worldbank.org/v2/country/all/indicator/${ind}?format=json&per_page=20000&date=2000:2025" \
        "$RAW/worldbank/${ind}.json" || true
done

echo
echo "=== BIS — credit to non-financial sector + debt service ratios ==="
fetch "https://www.bis.org/statistics/full_data_sets/bis_total_credit_csv.zip" "$RAW/bis/total_credit.zip" || true
fetch "https://www.bis.org/statistics/full_data_sets/dsr/full_BIS_DSR_csv.zip" "$RAW/bis/debt_service_ratios.zip" || true
fetch "https://www.bis.org/statistics/full_data_sets/cbs/full_BIS_CBS_csv.zip" "$RAW/bis/consolidated_banking.zip" || true
fetch "https://www.bis.org/statistics/full_data_sets/lbs/full_BIS_LBS_csv.zip" "$RAW/bis/locational_banking.zip" || true
fetch "https://www.bis.org/statistics/full_data_sets/cnfs/full_BIS_CNFS_csv.zip" "$RAW/bis/credit_to_non_financial.zip" || true

echo
echo "=== OECD — selected datasets via SDMX ==="
declare -a OECD=(
  "OECD.SDD.NAD,DSD_NAMAIN10@DF_TABLE1,1.0/all?dimensionAtObservation=AllDimensions:national_accounts"
  "OECD.SDD.STES,DSD_KEI@DF_KEI,1.0/all:key_economic_indicators"
)
for entry in "${OECD[@]}"; do
  key="${entry%%:*}"
  name="${entry##*:}"
  fetch "https://sdmx.oecd.org/public/rest/data/${key}?format=csvfilewithlabels" "$RAW/oecd/${name}.csv" || true
done

echo
echo "=== Bank of England — bankstats CSV exports ==="
# BoE Statistical Interactive Database series (no auth)
declare -a BOE=(
  "LPMVQJW:UK_loans_to_PNFCs"
  "IUMSDOR:UK_household_secured_lending_rate"
  "IUMSESR:UK_household_unsecured_rate"
  "IUMSWLA:UK_loans_PNFCs_rate"
  "RPMVN0Z:UK_PNFC_writeoffs"
)
for entry in "${BOE[@]}"; do
  key="${entry%%:*}"
  name="${entry##*:}"
  url="https://www.bankofengland.co.uk/boeapps/database/_iadb-fromshowcolumns.asp?Travel=NIxAZxSUx&FromSeries=1&ToSeries=50&DAT=ALL&FNY=Y&CSVF=TT&html.x=66&html.y=26&SeriesCodes=${key}&UsingCodes=Y&Filter=N&title=${key}&VPD=Y"
  fetch "$url" "$RAW/boe/${name}.csv" || true
done

echo
echo "=== EBA Risk Dashboard (most recent public XLSX) ==="
# The EBA dashboard URL pattern changes per release; we fetch the catalogue page
# and then try the most-recent known direct URL.
fetch "https://www.eba.europa.eu/sites/default/documents/files/document_library/Risk%20Analysis%20and%20Data/Risk%20dashboard/Q4%202024/EBA%20Dashboard%20-%20Q4%202024.xlsx" \
      "$RAW/eba/risk_dashboard_q4_2024.xlsx" || true
fetch "https://www.eba.europa.eu/sites/default/documents/files/document_library/Risk%20Analysis%20and%20Data/Risk%20dashboard/Q3%202024/EBA%20Dashboard%20-%20Q3%202024.xlsx" \
      "$RAW/eba/risk_dashboard_q3_2024.xlsx" || true

echo
echo "=== SEC EDGAR — Financial Statement Data Sets (small slice) ==="
# Quarterly FSDS bundles. We pull the most recent two quarters as a sample.
fetch "https://www.sec.gov/files/dera/data/financial-statement-data-sets/2024q4.zip" \
      "$RAW/sec_edgar/2024q4.zip" || true
fetch "https://www.sec.gov/files/dera/data/financial-statement-data-sets/2024q3.zip" \
      "$RAW/sec_edgar/2024q3.zip" || true

echo
echo "=== Loan-level samples ==="
# Lending Club historical data — Wharton mirror (academic, no auth)
fetch "https://files.consumerfinance.gov/ccdb/complaints.csv.zip" \
      "$RAW/loan_level/cfpb_consumer_complaints.csv.zip" || true

# Freddie Mac sample loan-level data — sample dataset is publicly downloadable
fetch "https://www.freddiemac.com/fmac-resources/research/pdf/historical-loan-level-data.pdf" \
      "$RAW/loan_level/freddie_mac_methodology.pdf" || true

echo
echo "=== Public PDFs — default & recovery studies ==="
# Moody's annual default study and S&P annual default reports — public PDFs.
# URLs change yearly; try a known historical path.
fetch "https://www.moodys.com/sites/products/ProductAttachments/2023%20Default%20Study.pdf" \
      "$RAW/credit_pdfs/moodys_2023_default_study.pdf" || true

# IMF Global Financial Stability Report (most recent autumn edition)
fetch "https://www.imf.org/-/media/Files/Publications/GFSR/2024/October/English/text.ashx" \
      "$RAW/credit_pdfs/imf_gfsr_2024_oct.pdf" || true

# ECB Financial Stability Review
fetch "https://www.ecb.europa.eu/pub/financial-stability/fsr/html/ecb.fsr202411~9c5d04ca7d.en.html" \
      "$RAW/credit_pdfs/ecb_fsr_2024_nov.html" || true

echo
echo "=== Done. ==="
echo "Raw data is under data/raw/. Run data/scripts/postprocess.sh to extract zips and convert formats."
