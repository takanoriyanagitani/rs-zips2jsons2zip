use std::io;

use io::BufRead;
use io::Read;

use io::Seek;

use io::Write;

use std::fs::File;

use serde_json::Map;
use serde_json::Value;

use flate2::bufread::GzDecoder;

use zip::ZipArchive;
use zip::ZipWriter;

use zip::read::ZipFile;

use zip::write::SimpleFileOptions;

pub use flate2;
pub use serde_json;
pub use zip;

pub fn slice2jobj(s: &[u8]) -> Result<Map<String, Value>, io::Error> {
    serde_json::from_slice(s).map_err(io::Error::other)
}

pub fn slice2jobj_mapd<O, M>(
    mut original: O,
    mapper: M,
) -> impl FnMut(&[u8]) -> Result<Map<String, Value>, io::Error>
where
    O: FnMut(&[u8]) -> Result<Map<String, Value>, io::Error>,
    M: Fn(Map<String, Value>) -> Map<String, Value>,
{
    move |s: &[u8]| {
        let m: Map<_, _> = original(s)?;
        let mapd: Map<_, _> = mapper(m);
        Ok(mapd)
    }
}

/// Gets jsons from zip items in the zip archive.
pub fn zip2objects<R, P>(
    mut zfile: ZipArchive<R>,
    slice2json: &mut P,
    buf: &mut Vec<u8>,
) -> impl Iterator<Item = Result<Map<String, Value>, io::Error>>
where
    R: Read + Seek,
    P: FnMut(&[u8]) -> Result<Map<String, Value>, io::Error>,
{
    let sz: usize = zfile.len();

    let mut ix: usize = 0;
    std::iter::from_fn(move || {
        let size_check: bool = ix < sz;
        if !size_check {
            return None;
        }

        let rmap: Result<Map<String, Value>, _> = zfile
            .by_index(ix)
            .map_err(io::Error::other)
            .and_then(|mut zitem: ZipFile<_>| {
                buf.clear();
                io::copy(&mut zitem, buf)?;
                slice2json(buf)
            });

        ix += 1;
        Some(rmap)
    })
}

/// Serializes json objects.
pub fn maps2buf<I>(jobjs: I, mut buf: &mut Vec<u8>) -> Result<(), io::Error>
where
    I: Iterator<Item = Result<Map<String, Value>, io::Error>>,
{
    buf.clear();
    for rjobj in jobjs {
        let jobj: Map<_, _> = rjobj?;
        serde_json::to_writer(&mut buf, &jobj)?;
        writeln!(&mut buf)?;
    }
    Ok(())
}

/// Writes the serialized json as a zip item.
pub fn jsons2zip<W, E>(
    jsons: &[u8],
    zwtr: &mut ZipWriter<W>,
    opts: SimpleFileOptions,
    name: &str,
    buf: &mut Vec<u8>,
    encoder: &E,
) -> Result<(), io::Error>
where
    W: Write + Seek,
    E: Fn(&[u8], &mut Vec<u8>) -> Result<(), io::Error>,
{
    zwtr.start_file(name, opts)?;

    buf.clear();
    encoder(jsons, buf)?;

    zwtr.write_all(buf)?;

    Ok(())
}

pub fn slice2zcat2jobj_new(
    mut buf: Vec<u8>,
) -> impl FnMut(&[u8]) -> Result<Map<String, Value>, io::Error> {
    move |jsongz: &[u8]| {
        let mut dec: GzDecoder<_> = GzDecoder::new(jsongz);
        buf.clear();
        io::copy(&mut dec, &mut buf)?;
        let json: &[u8] = &buf;
        serde_json::from_slice(json).map_err(io::Error::other)
    }
}

pub fn basename2jsonlname<W>(bname: &str, w: &mut W) -> Result<(), io::Error>
where
    W: std::fmt::Write,
{
    write!(w, "{bname}.jsonl").map_err(io::Error::other)
}

/// Creates item name from the zip filename.
///
/// - input: basename.zip
/// - output: basename.jsonl
pub fn zipfilename2itemname(zname: &str, itemname: &mut String) {
    let sz: usize = zname.len();
    let neo: usize = sz - 4; // ".zip"
    let noext: &str = &zname[..neo];
    itemname.clear();
    basename2jsonlname(noext, itemname).ok();
}

/// Just use the serialized json as the output.
pub fn json2bytes_nop(mut serialized_json: &[u8], output: &mut Vec<u8>) -> Result<(), io::Error> {
    output.clear();
    io::copy(&mut serialized_json, output)?;
    Ok(())
}

pub fn zipnames2zips2jsons2zip<I, P, W, E, N>(
    zipnames: I,
    mut slice2json: P,
    mut zwtr: ZipWriter<W>,
    opts: SimpleFileOptions,
    zname2iname: N,
    json2bytes: E,
) -> Result<(), io::Error>
where
    I: Iterator<Item = String>,
    P: FnMut(&[u8]) -> Result<Map<String, Value>, io::Error>,
    W: Write + Seek,
    E: Fn(&[u8], &mut Vec<u8>) -> Result<(), io::Error>,
    N: Fn(&str, &mut String),
{
    let mut buf_zfile2jobj: Vec<u8> = vec![];
    let mut buf_serialized_jsons: Vec<u8> = vec![];
    let mut buf_name: String = String::default();
    let mut buf_enc: Vec<u8> = vec![];

    for zname in zipnames {
        let f: File = File::open(&zname)?;
        let zfile: ZipArchive<_> = ZipArchive::new(f)?;
        let jsons = zip2objects(zfile, &mut slice2json, &mut buf_zfile2jobj);
        maps2buf(jsons, &mut buf_serialized_jsons)?;
        zname2iname(&zname, &mut buf_name);
        jsons2zip(
            &buf_serialized_jsons,
            &mut zwtr,
            opts,
            &buf_name,
            &mut buf_enc,
            &json2bytes,
        )?;
    }
    let mut w: W = zwtr.finish()?;
    w.flush()
}

pub fn stdin2names() -> impl Iterator<Item = String> {
    io::stdin().lock().lines().map_while(Result::ok)
}

pub fn stdin2znames2zips2jsons2zip_default(outfile: File) -> Result<(), io::Error> {
    let names = stdin2names();
    let zwtr: ZipWriter<_> = ZipWriter::new(outfile);
    let opts: SimpleFileOptions = SimpleFileOptions::default();
    zipnames2zips2jsons2zip(
        names,
        slice2jobj,
        zwtr,
        opts,
        zipfilename2itemname,
        json2bytes_nop,
    )
}
