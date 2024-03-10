use super::*;
use std::fs;
use std::fs::File;
use std::io::{BufRead, Cursor, Seek, SeekFrom, Write};
use regex::Regex;

pub fn extract(
    format: &Format,
    archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
) -> Result<HashMap<String, Vec<u8>>> {
    // let fp_folder_str = format!("{}.pretty", format.name);
    let fp_folder_str = "SamacSys_Parts.pretty";
    let td_folder_str = "SamacSys_Parts.3dshapes";

    //ensure we have the footprint library folder
    let footprint_folder = PathBuf::from(&format.output_path).join(fp_folder_str);
    let td_folder = PathBuf::from(&format.output_path).join(td_folder_str);
    if !footprint_folder.exists() {
        fs::create_dir_all(footprint_folder.clone())?;
    }
    if !td_folder.exists() {
        fs::create_dir_all(td_folder.clone())?;
    }

    //ensure the symbol library exists
    let fn_lib = PathBuf::from(&format.output_path).join("SamacSys_Parts.kicad_sym");

    if !fn_lib.exists() {
        fs::write(
            &fn_lib,
            "(kicad_symbol_lib (version 20211014) (generator SamacSys_ECAD_Model)\r\n)\r\n",
        )
        .expect("Unable to create symbol library file");
    }

    let mut symbols: Vec<String> = Vec::new();

    // define regex pattern
    let pattern = r#"\(model (\w+\.stp)"#;
    // create regex object
    let re = Regex::new(pattern).unwrap();

    for i in 0..archive.len() {
        let mut item = archive.by_index(i)?;
        let name = item.name();
        let path = PathBuf::from(name);
        let base_name = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(ext) = &path.extension() {
            match ext.to_str() {
                //footprint and 3d files are copied first
                Some("kicad_mod") => {
                    let mut f_data = Vec::<u8>::new();
                    item.read_to_end(&mut f_data)?;
                    let f_data_str = String::from_utf8_lossy(&f_data);
                    let f_data_str_replaced = re.replace_all(&f_data_str, |caps: &regex::Captures| {
                        format!("(model \"../SamacSys_Parts.3dshapes/{}\"", &caps[1])
                    });
                    
                    let mut f_data_replaced = Vec::<u8>::new();
                    f_data_replaced.extend(f_data_str_replaced.bytes()); 

                    let mut f = File::create(footprint_folder.join(base_name))?;
                    f.write_all(&f_data_replaced)?;
                }
                Some("stl") | Some("stp") | Some("wrl") => {
                    let mut f_data = Vec::<u8>::new();
                    item.read_to_end(&mut f_data)?;
                    let mut f = File::create(td_folder.join(base_name))?;
                    f.write_all(&f_data)?;
                }
                Some("kicad_sym") => {
                    //save these to add later, so KiCad will be able to load the footprints right away
                    symbols.push(name.to_owned());
                }
                _ => {
                    // ignore all other files
                }
            }
        }
    }

    let mut f = File::options().read(true).write(true).open(&fn_lib)?;
    f.seek(SeekFrom::End(-3))?;

    for symbol_file in symbols {
        let mut f_data = Vec::<u8>::new();
        let mut item = archive.by_name(&symbol_file)?;
        item.read_to_end(&mut f_data)?;
        let mut lines: Vec<String> = (&f_data[..])
            .lines()
            .map(|l| l.expect("Could not parse line"))
            .collect();
        let end = &lines.len() - 1;

        for line in &lines[1..end] {
            f.write_all(line.as_bytes())?;
            f.write_all("\r\n".as_bytes())?;
        }
    }
    f.write_all(")\r\n".as_bytes())?;

    Ok(Files::new())
}
