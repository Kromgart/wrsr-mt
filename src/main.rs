use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

use regex::Regex;

mod cfg;
mod data;
mod input;
mod output;
mod nmf;

use cfg::APP_SETTINGS;




fn main() {
/*
    let test = fs::read_to_string(r"z:\wrsr-mg\pack\7L\model.patch").unwrap();
    let res = ModelPatch::from(&test);
    println!("{}", res);
    return;
*/

    match &APP_SETTINGS.command {
        cfg::AppCommand::Install(cfg::InstallCommand{ source, destination, is_check }) => {

            println!("Installing from source: {}", source.to_str().unwrap());
            assert!(source.exists(), "Pack source directory does not exist!");

            println!("Installing to:          {}", destination.to_str().unwrap());
            assert!(destination.exists(), "Destination directory does not exist.");
            
            println!("Stock game files:       {}", APP_SETTINGS.path_stock.to_str().unwrap());
            assert!(APP_SETTINGS.path_stock.exists(), "Stock game files directory does not exist.");

            println!("Workshop directory:     {}", APP_SETTINGS.path_workshop.to_str().unwrap());
            assert!(APP_SETTINGS.path_workshop.exists(), "Workshop directory does not exist.");


            let mut pathbuf: PathBuf = APP_SETTINGS.path_stock.clone();
            pathbuf.push("buildings");
            pathbuf.push("buildingtypes.ini");

            let stock_buildings_ini = fs::read_to_string(&pathbuf).unwrap();
            let mut stock_buildings = { 
                let mut mp = HashMap::with_capacity(512);
                let rx = Regex::new(r"\$TYPE ([_[:alnum:]]+?)\r\n((?s).+?\n END\r\n)").unwrap();

                for caps in rx.captures_iter(&stock_buildings_ini) {
                    let key = caps.get(1).unwrap().as_str();
                    mp.insert(
                        key, 
                        (key, data::StockBuilding::Unparsed(caps.get(2).unwrap().as_str()))
                    );
                }
                
                mp
            };

            println!("Found {} stock buildings", stock_buildings.len());

            pathbuf.push(source);
            println!("Reading sources...");
            let data = input::read_validate_sources(pathbuf.as_path(), &mut stock_buildings);
            println!("Sources verified.");

            if *is_check {
                println!("Check complete.");
            } else {
                println!("Creating mods...");
                pathbuf.push(destination);

                output::generate_mods(pathbuf.as_path(), data);
            }
        },


        cfg::AppCommand::Nmf(cmd) => {
            match cmd {
                cfg::NmfCommand::Show(cfg::NmfShowCommand { path, .. }) => {
                    // TODO: with patch option

                    let buf = fs::read(path).expect("Cannot read nmf file at the specified path");
                    let (nmf, rest) = nmf::Nmf::parse_bytes(buf.as_slice()).expect("Failed to parse the model nmf");

                    println!("{}\n", nmf);

                    let unused: Vec<_> = nmf.get_unused_submaterials().map(|sm| &sm.name).collect();
                    if unused.len() > 0 {
                        print!("WARNING: has unused materials [ ");
                        for sm in unused {
                            print!("{}; ", sm);
                        }
                        println!("]\n");
                    }

                    assert_eq!(rest.len(), 0, "Model nmf parsed with leftovers ({} bytes)", rest.len());
                },
                cfg::NmfCommand::Patch(_) => {
                    // TODO
                    todo!("nmf patch is not implemented")
                }
            }
        }
    };
}
