use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use const_format::concatcp;

mod nmf;
mod ini;

mod building_def;
mod modpack;

mod cfg;

//mod data;
//mod input;
//mod output;


use cfg::{AppSettings, APP_SETTINGS, RENDERCONFIG_INI, BUILDING_INI};


fn main() {
    //modpack::make_relative_token(&p_from, &p_to));

    match &APP_SETTINGS.command {
        cfg::AppCommand::Modpack(cmd) => {
            print_dirs();

            match cmd {
                cfg::ModpackCommand::Install(cfg::ModpackInstallCommand { source, destination }) => {
                    println!("Installing from source: {}", source.display());
                    assert!(source.exists(), "Modpack source directory does not exist!");
                    println!("Reading modpack sources...");

                    match modpack::read_validate_sources(source.as_path()) {
                        Ok((buildings, skins_count)) => {
                            println!("Found {} buildings, {} skins", buildings.len(), skins_count);
                            let max_buildings = AppSettings::MAX_BUILDINGS - (skins_count / AppSettings::MAX_SKINS_IN_MOD + 1) * AppSettings::MAX_BUILDINGS_IN_MOD;
                            assert!(buildings.len() < max_buildings, "Too many building sources");
                            println!("Installing to {}...", destination.display());
                            assert!(destination.exists(), "Destination directory does not exist");

                            let mut log_path = destination.to_path_buf();
                            log_path.push(modpack::MODPACK_LOG);
                            if log_path.exists() {
                                panic!("Cannot proceed: target directory has {}, which indicates that a modpack has already been installed here.", modpack::MODPACK_LOG);
                            }

                            let log_file = fs::OpenOptions::new().write(true).create_new(true).open(log_path).expect("Cannot create log file");
                            let mut log_file = std::io::BufWriter::new(log_file);

                            modpack::install(buildings, destination, &mut log_file);

                            log_file.flush().unwrap();
                            println!("Modpack installed");
                        },
                        Err(e) => {
                            panic!("FAILED: encountered {} errors when reading sources", e);
                        }
                    }
                },
                cfg::ModpackCommand::Validate(source) => {
                    println!("Validating modpack at {}", source.display());
                    assert!(source.exists(), "Modpack source directory does not exist!");
                    println!("Reading modpack sources...");

                    match modpack::read_validate_sources(source.as_path()) {
                        Ok((buildings, skins_count)) => {
                            println!("OK: found {} buildings, {} skins", buildings.len(), skins_count);
                        },
                        Err(e) => {
                            eprintln!("FAILED: encountered {} errors", e);
                        }
                    }
                },
            }
        },


        cfg::AppCommand::Nmf(cmd) => {
            match cmd {
                cfg::NmfCommand::Show(path) => {
                    let nmf = nmf::NmfInfo::from_path(path).expect("Failed to read the nmf file");
                    println!("{}", nmf);
                },

                cfg::NmfCommand::ToObj(cfg::FromToCommand { input, output }) => {
                    let nmf = nmf::NmfBufFull::from_path(input).expect("Failed to read the nmf file");

                    let f_out = fs::OpenOptions::new()
                                    .write(true)
                                    .create_new(true)
                                    .open(output)
                                    .expect("Cannot create output file");

                    let mut wr = std::io::BufWriter::new(f_out);

                    use nmf::object_full::{RawVertex, RawPoint};
                    let mut vx_map = ahash::AHashMap::<&RawVertex, usize>::with_capacity(0);
                    let mut n1_map = ahash::AHashMap::<&RawVertex, usize>::with_capacity(0);
                    let mut uv_map = ahash::AHashMap::<&RawPoint, usize>::with_capacity(0);
                    let mut vx_vec = Vec::<usize>::with_capacity(0);
                    let mut n1_vec = Vec::<usize>::with_capacity(0);
                    let mut uv_vec = Vec::<usize>::with_capacity(0);

                    let mut d_vx = 0_usize;
                    let mut d_n1 = 0_usize;
                    let mut d_uv = 0_usize;

                    for obj in nmf.objects.iter() {
                        let verts = obj.vertices();

                        vx_map.clear();
                        n1_map.clear();
                        uv_map.clear();

                        vx_vec.clear();
                        n1_vec.clear();
                        uv_vec.clear();

                        vx_map.reserve(verts.len());
                        n1_map.reserve(verts.len());
                        uv_map.reserve(verts.len());

                        vx_vec.reserve(verts.len());
                        n1_vec.reserve(verts.len());
                        uv_vec.reserve(verts.len());

                        writeln!(wr, "o {}", obj.name()).unwrap();

                        for v in verts {
                            if !vx_map.contains_key(v) {
                                d_vx += 1;
                                vx_map.insert(v, d_vx);
                                vx_vec.push(d_vx);
                                writeln!(wr, "v {:.6} {:.6} {:.6}", v.x, v.y, v.z).unwrap();
                            } else {
                                vx_vec.push(vx_map[v]);
                            }
                        }

                        let uvs = obj.uv_map();
                        for uv in uvs {
                            if !uv_map.contains_key(uv) {
                                d_uv += 1;
                                uv_map.insert(uv, d_uv);
                                uv_vec.push(d_uv);
                                writeln!(wr, "vt {:.6} {:.6}", uv.x, 1f32 - uv.y).unwrap();
                            } else {
                                uv_vec.push(uv_map[uv]);
                            }
                        }

                        let ns = obj.normals_1();
                        for n in ns {
                            if !n1_map.contains_key(n) {
                                d_n1 += 1;
                                n1_map.insert(n, d_n1);
                                n1_vec.push(d_n1);
                                writeln!(wr, "vn {:.6} {:.6} {:.6}", n.x, n.y, n.z).unwrap();
                            } else {
                                n1_vec.push(n1_map[n]);
                            }
                        }

                        writeln!(wr, "s off").unwrap();

                        for f in obj.faces() {
                            let v1  = vx_vec[f.v1 as usize];
                            let n1  = n1_vec[f.v1 as usize];
                            let uv1 = uv_vec[f.v1 as usize];

                            let v2  = vx_vec[f.v2 as usize];
                            let n2  = n1_vec[f.v2 as usize];
                            let uv2 = uv_vec[f.v2 as usize];

                            let v3  = vx_vec[f.v3 as usize];
                            let n3  = n1_vec[f.v3 as usize];
                            let uv3 = uv_vec[f.v3 as usize];

                            write!(wr, "f {}/{}/{}",   v1, uv1, n1).unwrap();
                            write!(wr, "  {}/{}/{}",   v2, uv2, n2).unwrap();
                            write!(wr, "  {}/{}/{}\n", v3, uv3, n3).unwrap();
                        }
                    }

                    wr.flush().expect("Failed flushing the output");
                    println!("Done");
                },

                cfg::NmfCommand::Scale(cfg::ScaleCommand { input, factor, output }) => {
                    let mut nmf = nmf::NmfBufFull::from_path(input).expect("Failed to read the nmf file");
                    for o in nmf.objects.iter_mut() {
                        o.scale(*factor);
                    }
                    nmf.write_to_file(output).unwrap();
                    println!("Done");
                },

                cfg::NmfCommand::Mirror(cfg::FromToCommand { input, output }) => {
                    let mut nmf = nmf::NmfBufFull::from_path(input).expect("Failed to read the nmf file");
                    for o in nmf.objects.iter_mut() {
                        o.mirror_z();
                    }
                    nmf.write_to_file(output).unwrap();
                    println!("Done");
                },

                cfg::NmfCommand::Optimize(cfg::FromToCommand { input, output }) => {
                    let mut nmf = nmf::NmfBufFull::from_path(input).expect("Failed to read the nmf file");
                    for o in nmf.objects.iter_mut() {
                        o.optimize_indices();
                    }
                    nmf.write_to_file(output).unwrap();
                    println!("Done");
                },
            }
        },


        //---------------- mod subcommand --------------------------------
        cfg::AppCommand::ModBuilding(cmd) => {
            use building_def::ModBuildingDef;

            fn check_and_copy_building(dir_input: &PathBuf, dir_output: &PathBuf) -> ModBuildingDef {
                let render_ini = dir_input.join(RENDERCONFIG_INI);
                let bld_ini = dir_input.join(BUILDING_INI);
                let bld_def = ModBuildingDef::from_render_path(&bld_ini, &render_ini, ini::normalize_join, false)
                    .expect("Cannot parse building");

                {
                    let check_path = |path: &Path| assert!(path.starts_with(dir_input), 
                                          "To update the whole building in one operation, all potentially modified files (building.ini, \
                                          renderconfig.ini, *.nmf) must be located in the input directory. Otherwise you should update \
                                          files individually, one-by-one (using appropriate commands).");

                    let check_path_opt = |opt: &Option<PathBuf>| if let Some(p) = opt.as_ref() { check_path(p) };

                    check_path(&bld_def.render);
                    check_path(&bld_def.building_ini);
                    check_path(&bld_def.model);
                    check_path_opt(&bld_def.model_lod);
                    check_path_opt(&bld_def.model_lod2);
                    check_path_opt(&bld_def.model_e);
                }

                println!("Building parsed successfully. Copying files...");
                let bld_def = bld_def.shallow_copy_to(dir_output).expect("Cannot copy building files");
                println!("Files copied.");
                bld_def
            }

            macro_rules! modify_ini {
                ($buf:ident, $path:expr, $name:expr, $parser:expr, $modifier:expr $(, $m_p:expr)*) => {{
                    read_to_string_buf($path, &mut $buf).expect(concatcp!("Cannot read ", $name));
                    let mut ini = $parser(&mut $buf).expect(concatcp!("Cannot parse ", $name));
                    $modifier(&mut ini $(, $m_p)*);
                    let mut out_writer = io::BufWriter::new(fs::OpenOptions::new().write(true).truncate(true).open($path).unwrap());
                    ini.write_to(&mut out_writer).unwrap();
                    out_writer.flush().unwrap();
                    println!("{}: OK", $name);
                }};
            }

            fn modify_models<F: Fn(&mut nmf::ObjectFull)>(bld_def: &ModBuildingDef, pfx: &Path, obj_modifier: F) {
                let modify_nmf = |path: Option<&PathBuf>| {
                    if let Some(path) = path {
                        let mut nmf = nmf::NmfBufFull::from_path(path).expect("Failed to read the nmf file");
                        for o in nmf.objects.iter_mut() {
                            obj_modifier(o);
                        }

                        nmf.write_to_file(path).expect("Failed to write the updated nmf");
                        println!("{}: OK", path.strip_prefix(pfx).unwrap().display());
                    }
                };

                modify_nmf(Some(&bld_def.model));
                modify_nmf(bld_def.model_lod.as_ref());
                modify_nmf(bld_def.model_lod2.as_ref());
                modify_nmf(bld_def.model_e.as_ref());
            }


            match cmd {
                cfg::ModCommand::Validate(dir_input) => {
                    let bld_ini = dir_input.join(BUILDING_INI);
                    let render_ini = dir_input.join(RENDERCONFIG_INI);
                    match building_def::ModBuildingDef::from_render_path(&bld_ini, &render_ini, ini::normalize_join, true) {
                        Ok(bld) => {
                            println!("{}\nOK", bld);
                        },
                        Err(e) => {
                            eprintln!("Building has errors:\n{}", e);
                            std::process::exit(1);
                        }
                    }
                },

                cfg::ModCommand::Scale(cfg::ScaleCommand { input: dir_input, factor, output: dir_output }) => {

                    let bld_def = check_and_copy_building(dir_input, dir_output);
                    println!("Updating...");

                    let mut buf = String::with_capacity(16 * 1024);
                    modify_ini!(buf, &bld_def.building_ini, BUILDING_INI,     ini::parse_building_ini,     ini::transform::scale_building, *factor);
                    modify_ini!(buf, &bld_def.render,       RENDERCONFIG_INI, ini::parse_renderconfig_ini, ini::transform::scale_render,   *factor);
                    modify_models(&bld_def, dir_output, |o| o.scale(*factor));
                },
                cfg::ModCommand::Mirror(cfg::FromToCommand { input: dir_input, output: dir_output }) => {
                    let bld_def = check_and_copy_building(dir_input, dir_output);
                    println!("Updating...");

                    let mut buf = String::with_capacity(16 * 1024);
                    modify_ini!(buf, &bld_def.building_ini, BUILDING_INI,     ini::parse_building_ini,     ini::transform::mirror_z_building);
                    modify_ini!(buf, &bld_def.render,       RENDERCONFIG_INI, ini::parse_renderconfig_ini, ini::transform::mirror_z_render);
                    modify_models(&bld_def, dir_output, |o| o.mirror_z());
                },
            }
        },


        //---------------- ini subcommand --------------------------------
        cfg::AppCommand::Ini(cmd) => {

            fn process_tokens<T: std::fmt::Display>(ts: Vec<(&str, ini::common::ParseResult<T>)>) {
                for (t_str, t_val) in ts.iter() {
                    match t_val {
                        Ok((t, rest)) => {
                            print!("{}", t);
                            if let Some(rest) = rest {
                                print!(" [remainder: {:?}]", rest);
                            }
                            println!();
                        },
                        Err(e) => println!(" > > > Error > > >\n > > > {}\n > > > chunk: [{}]", e, t_str),
                    }
                }
            }

            fn save_ini_as<U: ini::IniToken>(path: &Path, ini: ini::IniFile<U>) {
                let out_writer = io::BufWriter::new(fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap());
                ini.write_to(out_writer).expect("Could not write modified file");
                println!("Done. File saved as {}", path.display());
            }

            match cmd {
                cfg::IniCommand::ParseBuilding(path) => {
                    let buf = fs::read_to_string(path).expect("Cannot read the specified file");
                    let tokens = ini::parse_building_tokens(&buf);
                    process_tokens(tokens);
                },
                cfg::IniCommand::ParseRender(path) => {
                    let buf = fs::read_to_string(path).expect("Cannot read the specified file");
                    let tokens = ini::parse_render_tokens(&buf);
                    process_tokens(tokens);
                },
                cfg::IniCommand::ParseMtl(path) => {
                    let buf = fs::read_to_string(path).expect("Cannot read the specified file");
                    let tokens = ini::parse_material_tokens(&buf);
                    process_tokens(tokens);
                },
                cfg::IniCommand::ScaleBuilding(cfg::ScaleCommand { input, factor, output }) => {
                    let file = fs::read_to_string(input).expect("Cannot read the specified file");
                    let mut ini = ini::parse_building_ini(&file).expect("Cannot parse building.ini");
                    ini::transform::scale_building(&mut ini, *factor);
                    save_ini_as(output, ini);
                },
                cfg::IniCommand::ScaleRender(cfg::ScaleCommand { input, factor, output }) => {
                    let file = fs::read_to_string(input).expect("Cannot read the specified file");
                    let mut ini = ini::parse_renderconfig_ini(&file).expect("Cannot parse renderconfig");
                    ini::transform::scale_render(&mut ini, *factor);
                    save_ini_as(output, ini);
                },
                cfg::IniCommand::MirrorBuilding(cfg::FromToCommand { input, output }) => {
                    let file = fs::read_to_string(input).expect("Cannot read the specified file");
                    let mut ini = ini::parse_building_ini(&file).expect("Cannot parse building.ini");
                    ini::transform::mirror_z_building(&mut ini);
                    save_ini_as(output, ini);
                },
                cfg::IniCommand::MirrorRender(cfg::FromToCommand { input, output }) => {
                    let file = fs::read_to_string(input).expect("Cannot read the specified file");
                    let mut ini = ini::parse_renderconfig_ini(&file).expect("Cannot parse renderconfig");
                    ini::transform::mirror_z_render(&mut ini);
                    save_ini_as(output, ini);
                }
            }

        },

        //---------------- subcommands end --------------------------------
    };
}


fn print_dirs() {
    println!("Stock game files:   {}", APP_SETTINGS.path_stock.as_path().display());
    assert!(APP_SETTINGS.path_stock.exists(), "Stock game files directory does not exist.");

    println!("Workshop directory: {}", APP_SETTINGS.path_workshop.as_path().display());
    assert!(APP_SETTINGS.path_workshop.exists(), "Workshop directory does not exist.");
}


pub fn read_to_buf(path: &Path, buf: &mut Vec<u8>) -> Result<(), std::io::Error> {
    use std::io::Read;
    use std::convert::TryInto;
    buf.clear();

    let mut file = fs::File::open(path)?;
    let meta = file.metadata()?;
    let sz: usize = meta.len().try_into().expect("Cannot get file length");
    buf.reserve(sz);
    file.read_to_end(buf)?;
    Ok(())
}


pub fn read_to_string_buf<P: AsRef<Path>>(path: P, buf: &mut String) -> Result<(), std::io::Error> {
    use std::io::Read;
    use std::convert::TryInto;
    buf.clear();

    let mut file = fs::File::open(path)?;
    let meta = file.metadata()?;
    let sz: usize = meta.len().try_into().expect("Cannot get file length");
    buf.reserve(sz);
    file.read_to_string(buf)?;
    Ok(())
}
