use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use std::convert::TryFrom;

use regex::Regex;

mod nmf;
mod cfg;
mod data;
mod input;
mod output;

use cfg::APP_SETTINGS;
use nmf::Nmf;


fn main() {

    println!("Stock game files:   {}", APP_SETTINGS.path_stock.to_str().unwrap());
    assert!(APP_SETTINGS.path_stock.exists(), "Stock game files directory does not exist.");

    println!("Workshop directory: {}", APP_SETTINGS.path_workshop.to_str().unwrap());
    assert!(APP_SETTINGS.path_workshop.exists(), "Workshop directory does not exist.");

    match &APP_SETTINGS.command {
        cfg::AppCommand::Install(cfg::InstallCommand{ source, destination, is_check }) => {

            println!("Installing from source: {}", source.to_str().unwrap());
            assert!(source.exists(), "Pack source directory does not exist!");

            println!("Installing to:          {}", destination.to_str().unwrap());
            assert!(destination.exists(), "Destination directory does not exist.");
            
            let mut pathbuf: PathBuf = APP_SETTINGS.path_stock.clone();
            pathbuf.push("buildings");
            pathbuf.push("buildingtypes.ini");

            let stock_buildings_ini = fs::read_to_string(&pathbuf).expect("Stock buildings: cannot read buildingtypes.ini");
            let mut stock_buildings = { 
                let mut mp = HashMap::with_capacity(512);
                let rx = Regex::new(r"\$TYPE ([_[:alnum:]]+?)\r\n((?s).+?\n END\r\n)").expect("Stock buildings: cannot create parsing regex");

                for caps in rx.captures_iter(&stock_buildings_ini) {
                    let key = caps.get(1).unwrap().as_str();
                    let raw_value = caps.get(2).unwrap().as_str();
                    mp.insert(
                        key, 
                        (key, data::StockBuilding::Unparsed(raw_value))
                    );
                }
                
                mp
            };

            println!("Found {} stock buildings", stock_buildings.len());

            pathbuf.push(source);
            println!("Reading modpack sources...");

            match input::read_validate_sources(pathbuf.as_path(), &mut stock_buildings) {
                Ok(data) => {
                    println!("Modpack sources verified.");

                    if *is_check {
                        println!("Check complete.");
                    } else {
                        println!("Creating mods...");
                        pathbuf.push(destination);

                        output::generate_mods(pathbuf.as_path(), data);
                    }
                },
                Err(errs) => {
                    eprintln!("\nThe following {} errors were encountered when processing modpack sources:\n", errs.len());
                    for (i, e) in errs.iter().enumerate() {
                        eprintln!("{}) {}", i + 1, e);
                    }

                    eprintln!();

                    std::process::exit(1);
                }
            }
        },


        cfg::AppCommand::Nmf(cmd) => {
            match cmd {
                cfg::NmfCommand::Show(cfg::NmfShowCommand { path, with_patch }) => {

                    let buf = fs::read(path).expect("Cannot read nmf file at the specified path");
                    let (nmf, rest) = Nmf::parse_bytes(buf.as_slice()).expect("Failed to parse the model nmf");
                    if !rest.is_empty() {
                        println!("WARNING: nmf parsed with leftovers ({} bytes)", rest.len());
                    };

                    print_out(&nmf);

                    if let Some(p) = with_patch {
                        let buf_patch = fs::read_to_string(p).expect("Cannot read patch file at the specified path");
                        println!("Applying patch at {}...", p.to_str().unwrap());
                        let patch = data::ModelPatch::try_from(buf_patch.as_str()).unwrap();
                        println!("{}\n", &patch);

                        let patched = patch.apply(&nmf);
                        print_out(&patched);
                    }

                    //---------------------------------------------------
                    fn print_out(nmf: &Nmf) {
                        println!("{}", nmf);

                        let unused: Vec<_> = nmf.get_unused_submaterials()
                                                .map(|sm| &sm.name)
                                                .collect();

                        if !unused.is_empty() {
                            print!("\nWARNING: has unused materials [ ");
                            for sm in unused {
                                print!("{}; ", sm);
                            }
                            println!("]");
                        }
                    }
                },
                
                cfg::NmfCommand::Patch(cfg::NmfPatchCommand { input, patch, output }) => {
                    let buf = fs::read(input).expect("Cannot read nmf file at the specified path");
                    let (nmf, rest) = Nmf::parse_bytes(buf.as_slice()).expect("Failed to parse the model nmf");
                    if !rest.is_empty() {
                        panic!("Nmf parsed with leftovers ({} bytes)", rest.len());
                    };


                    let buf_patch = fs::read_to_string(patch).expect("Cannot read patch file at the specified path");
                    let patch = data::ModelPatch::try_from(buf_patch.as_str()).unwrap();

                    let patched = patch.apply(&nmf);

                    // NOTE: DEBUG
                    //println!("{}", &patched);

                    let file = std::fs::File::create(output).unwrap();
                    let mut writer = std::io::BufWriter::new(file);
                    patched.write_bytes(&mut writer);

                    println!("OK");
                }
            }
        }
    };
}
