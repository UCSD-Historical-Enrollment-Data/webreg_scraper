cd section || { echo "Error: cannot cd to 'section'"; exit 1; }
rm *.csv
cd ../overall || { echo "Error: cannot cd to 'overall'"; exit 1; }
rm *.csv