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

# SEC EDGAR requires a UA with contact info — keep it simple, no parentheses.
UA="Tyche Research security@tyche.network"

fetch() {
  local url="$1" out="$2"
  shift 2
  local extra=("$@")
  if [[ -f "$out" && "$FORCE" != "--force" ]]; then
    echo "  skip (exists): $(basename "$out")"
    return 0
  fi
  mkdir -p "$(dirname "$out")"
  if curl -fsSL --retry 3 --retry-delay 2 -A "$UA" ${extra[@]+"${extra[@]}"} -o "$out.tmp" "$url"; then
    mv "$out.tmp" "$out"
    local size; size=$(stat -f%z "$out" 2>/dev/null || stat -c%s "$out")
    echo "  ok   ($(numfmt --to=iec "$size" 2>/dev/null || echo "$size B")): $(basename "$out")"
  else
    rm -f "$out.tmp"
    echo "  FAIL: $(basename "$out") — $url" >&2
    return 1
  fi
}

echo "=== FRED — macro time series ==="
declare -a FRED=(
  GDPC1 CPIAUCSL DGS10 DFF BAMLH0A0HYM2 BAMLC0A0CM TEDRATE T10Y3M
  DRBLACBS DRSFRMACBS USREC UNRATE
  ECBESTRVOLWGTTRMDMNRT IRSTCB01EZQ156N CLVMNACSCAB1GQEA19 CP0000EZ19M086NEST
  CSCICP03EZM665S    # Euro area consumer confidence
  ECBDFR             # ECB deposit facility rate
  T10Y2Y             # 10y-2y term spread
  WTREGEN            # Treasury general account
)
for s in "${FRED[@]}"; do
  # Default endpoint returns the last ~3 years; we explicitly request full
  # history so calibration can reach Lehman 2008 and COVID 2020.
  fetch "https://fred.stlouisfed.org/graph/fredgraph.csv?id=$s&cosd=1900-01-01&coed=2026-12-31" \
        "$RAW/fred/${s}.csv" || true
done

echo
echo "=== ECB Statistical Data Warehouse ==="
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
  nama_10_gdp nama_10_a64 prc_hicp_manr ei_isen_m bs_bs7_q ert_bil_eur_a
  irt_lt_mcby_d sbs_sc_ovw bd_size_r3 fbs_pension_q
)
for ds in "${EUROSTAT[@]}"; do
  fetch "https://ec.europa.eu/eurostat/api/dissemination/sdmx/2.1/data/${ds}?format=SDMX-CSV&compressed=true" \
        "$RAW/eurostat/${ds}.csv.gz" || true
done

echo
echo "=== World Bank Open Data ==="
declare -a WB_INDICATORS=(
  NY.GDP.MKTP.CD FR.INR.RINR FS.AST.PRVT.GD.ZS GFDD.SI.02 GFDD.SI.03
  FS.AST.DOMS.GD.ZS CM.MKT.TRAD.GD.ZS
)
for ind in "${WB_INDICATORS[@]}"; do
  fetch "https://api.worldbank.org/v2/country/all/indicator/${ind}?format=json&per_page=20000&date=2000:2025" \
        "$RAW/worldbank/${ind}.json" || true
done

echo
echo "=== BIS — XLSX dashboards (the bulk-zip endpoints have moved) ==="
declare -a BIS=(
  "totcredit/totcredit.xlsx:total_credit_to_nfs"
  "dsr/dsr.xlsx:debt_service_ratios"
  "eer/broad.xlsx:nominal_effective_exchange_rates"
  "eer/narrow.xlsx:narrow_effective_exchange_rates"
  "rpp/rpp_long.xlsx:residential_property_prices"
  "cbs/d_a_b_a.xlsx:consolidated_banking_breakdown"
  "lbs/d_lbs_a.xlsx:locational_banking"
)
for entry in "${BIS[@]}"; do
  path="${entry%%:*}"
  name="${entry##*:}"
  fetch "https://www.bis.org/statistics/${path}" "$RAW/bis/${name}.xlsx" || true
done

echo
echo "=== OECD — SDMX REST with CSV accept header ==="
declare -a OECD=(
  "OECD.SDD.STES,DSD_KEI@DF_KEI/all:key_economic_indicators"
  "OECD.SDD.NAD,DSD_NAMAIN1@DF_QNA/all:quarterly_national_accounts"
  "OECD.SDD.STES,DSD_STES@DF_FINMARK/all:financial_market_trends"
)
for entry in "${OECD[@]}"; do
  key="${entry%%:*}"
  name="${entry##*:}"
  fetch "https://sdmx.oecd.org/public/rest/data/${key}" "$RAW/oecd/${name}.csv" \
        -H "Accept: application/vnd.sdmx.data+csv" || true
done

echo
echo "=== Bank of England — bankstats CSV exports ==="
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
echo "=== EBA Risk Dashboard — page changes per quarter, see manual fallback ==="
echo "(Skipping; EBA URL pattern shifts each release. See data/README.md for the manual link.)"

echo
echo "=== SEC EDGAR — Financial Statement Data Sets ==="
# The FSDS archive is now at /files/dera/data/financial-statement-data-sets/.
# We pull the most recent two quarters as a calibration sample (~250 MB total).
fetch "https://www.sec.gov/files/dera/data/financial-statement-data-sets/2024q4.zip" \
      "$RAW/sec_edgar/2024q4.zip" || true
fetch "https://www.sec.gov/files/dera/data/financial-statement-data-sets/2024q3.zip" \
      "$RAW/sec_edgar/2024q3.zip" || true

echo
echo "=== Public credit & default PDFs ==="
# Moody's / S&P annual default studies — public PDFs. URL paths shift; we try
# a couple of known patterns.
fetch "https://www.moodys.com/sites/products/ProductAttachments/2023%20Default%20Study.pdf" \
      "$RAW/credit_pdfs/moodys_2023_default_study.pdf" || true
fetch "https://www.spglobal.com/_assets/documents/ratings/research/101510568.pdf" \
      "$RAW/credit_pdfs/sp_2023_default_study.pdf" || true

# IMF Global Financial Stability Report
fetch "https://www.imf.org/-/media/Files/Publications/GFSR/2024/October/English/text.ashx" \
      "$RAW/credit_pdfs/imf_gfsr_2024_oct.pdf" || true

echo
echo "=== Loan-level / consumer-finance samples ==="
# CFPB Consumer Complaints database — large (~1.8 GB compressed) but it's the
# only fully public, no-auth, daily-refreshed dataset of US consumer-finance
# complaints with product / issue / company / state / response fields. Useful
# for stressing the schema, building UI fixtures, and doing NLP on dispute
# narratives. Set TYCHE_SKIP_BIG=1 to skip it on slow networks.
if [[ "${TYCHE_SKIP_BIG:-0}" != "1" ]]; then
  fetch "https://files.consumerfinance.gov/ccdb/complaints.csv.zip" \
        "$RAW/loan_level/cfpb_consumer_complaints.csv.zip" || true
else
  echo "  TYCHE_SKIP_BIG=1 — skipping CFPB complaints zip (~1.8 GB)"
fi

echo
echo "=== Done. ==="
echo "Raw data is under data/raw/. Run data/scripts/postprocess.sh to extract zips and convert formats."
