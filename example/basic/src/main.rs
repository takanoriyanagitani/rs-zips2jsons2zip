use std::process::ExitCode;

use std::collections::BTreeSet;

use std::io;

use std::fmt::Write;

use std::fs::File;

use rs_zips2jsons2zip::flate2;
use rs_zips2jsons2zip::serde_json;
use rs_zips2jsons2zip::zip;

use serde_json::Map;
use serde_json::Value;

use flate2::Compression;
use flate2::write::GzEncoder;

use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use rs_zips2jsons2zip::stdin2znames2zips2jsons2zip_default;
use rs_zips2jsons2zip::zipnames2zips2jsons2zip;

use rs_zips2jsons2zip::slice2jobj_mapd;
use rs_zips2jsons2zip::slice2zcat2jobj_new;
use rs_zips2jsons2zip::stdin2names;

fn env_val_by_key(key: &'static str) -> impl FnMut() -> Result<String, io::Error> {
    move || std::env::var(key).map_err(|e| io::Error::other(format!("env val {key} unknown: {e}")))
}

fn env2out_zipname() -> Result<String, io::Error> {
    env_val_by_key("ENV_OUTPUT_ZIP_FILENAME")()
}

fn enable_jsongz() -> bool {
    env_val_by_key("ENV_ENABLE_JSON_GZ")()
        .ok()
        .map(|s| s.eq("true"))
        .unwrap_or(false)
}

fn sub_default() -> Result<(), io::Error> {
    let out_zname: String = env2out_zipname()?;

    let f: File = File::create(out_zname)?;

    stdin2znames2zips2jsons2zip_default(f)?;

    Ok(())
}

/*
fn keep_filter(keep_keys: BTreeSet<String>) -> impl Fn(Map<String, Value>) -> Map<String, Value> {
    move |mut original: Map<_, _>| {
        original.retain(|key: &String, _val: &mut Value| keep_keys.contains(key));
        original
    }
}
*/

fn remove_filter(
    remove_keys: BTreeSet<String>,
) -> impl Fn(Map<String, Value>) -> Map<String, Value> {
    move |mut original: Map<_, _>| {
        original.retain(|key: &String, _val: &mut Value| !remove_keys.contains(key));
        original
    }
}

fn env2remove_keys_string() -> String {
    env_val_by_key("ENV_REMOVE_KEYS")().ok().unwrap_or_default()
}

fn env2remove_filter() -> impl Fn(Map<String, Value>) -> Map<String, Value> {
    let rkeys_string: String = env2remove_keys_string();
    let splited = rkeys_string.split(',');
    remove_filter(BTreeSet::from_iter(splited.map(|s| s.into())))
}

fn sub_jsongz() -> Result<(), io::Error> {
    let out_zname: String = env2out_zipname()?;

    let f: File = File::create(out_zname)?;

    let zipnames = stdin2names();

    let slice2zcat2jobj = slice2zcat2jobj_new(vec![]);
    let slice2json = slice2jobj_mapd(slice2zcat2jobj, env2remove_filter());

    let zwtr: ZipWriter<_> = ZipWriter::new(f);

    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    let zname2iname = |zfilename: &str, outname: &mut String| {
        outname.clear();

        let sz: usize = zfilename.len();
        let shorter: usize = sz - 4; // ".zip"
        let noext: &str = &zfilename[..shorter];
        let mut splited = noext.rsplitn(2, '/');
        let basename: &str = splited.next().unwrap_or_default();
        write!(outname, "{basename}.jsonl.gz").ok();
    };

    let json2bytes = |mut serialized_json: &[u8], buf: &mut Vec<u8>| {
        buf.clear();
        let mut genc = GzEncoder::new(buf, Compression::fast());
        io::copy(&mut serialized_json, &mut genc)?;
        io::Write::flush(&mut genc)?;
        genc.finish()?;
        Ok::<_, io::Error>(())
    };

    zipnames2zips2jsons2zip(zipnames, slice2json, zwtr, opts, zname2iname, json2bytes)
}

fn sub() -> Result<(), io::Error> {
    match enable_jsongz() {
        true => sub_jsongz(),
        false => sub_default(),
    }
}

fn main() -> ExitCode {
    sub().map(|_| ExitCode::SUCCESS).unwrap_or_else(|e| {
        eprintln!("{e}");
        ExitCode::FAILURE
    })
}
