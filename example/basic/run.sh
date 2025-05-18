#!/bin/sh

iz0="./sample.d/input0.zip"
iz1="./sample.d/input1.zip"

oz="/guest-w.d/output.zip"

mkdir -p ./sample-out.d

gen_input(){
	echo creating input zip files...

	mkdir -p ./sample.d

	jq -c -n '{name: "fuji",  height: 3.776}' > ./sample.d/z0j0.json
	jq -c -n '{name: "takao", height: 0.599}' > ./sample.d/z0j1.json

	jq -c -n '{name: "FUJI",  height: 3.776}' > ./sample.d/z1j0.json
	jq -c -n '{name: "TAKAO", height: 0.599}' > ./sample.d/z1j1.json

	find ./sample.d -type f -name 'z0j?.json' |
		zip -0 -@ -T -v -o "${iz0}"
	find ./sample.d -type f -name 'z1j?.json' |
		zip -0 -@ -T -v -o "${iz1}"

}

test -f "${iz0}" || gen_input
test -f "${iz1}" || gen_input

echo input files info...
unzip -lv "${iz0}"
unzip -lv "${iz1}"

echo
echo creating a zip file from zip files...
ls "${iz0}" "${iz1}" |
	sed \
		-n \
		-e 's/..sample.d/guest.d/' \
		-e 's,^,/,' \
		-e p |
	wazero \
		run \
		-env ENV_OUTPUT_ZIP_FILENAME="${oz}" \
		-mount "./sample-out.d:/guest-w.d" \
		-mount "./sample.d:/guest.d:ro" \
		./basic.wasm

echo
echo a zip file info
unzip -lv ./sample-out.d/output.zip

echo
echo json values in the zip file
unzip -p ./sample-out.d/output.zip
