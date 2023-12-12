use std::{cmp::Ordering, error::Error, fs, collections::{HashMap, VecDeque}};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::MERGE_DEPS;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Version {
    pub major: Option<u8>,
    pub minor: Option<u8>,
    pub patch: Option<u8>,
    pub appendix: Option<String>,
    pub patch_strategy: PatchStrategy,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone, Copy)]
pub enum PatchStrategy {
    None = 0,
    Patch = 1,
    Minor = 2,
    Major = 3,
}

impl From<Version> for String {
    fn from(value: Version) -> Self {
        let mut s = VecDeque::with_capacity(10);
        for mut v in [value.patch, value.minor, value.major].into_iter().flatten() {
            loop {
                let c = char::from_digit(v as u32 % 10, 10).unwrap();
                s.push_front(c);
                v /= 10;
                if v == 0 {
                    break;
                }
            }
            s.push_front('.');
        }
        s.pop_front();
        match value.patch_strategy {
            PatchStrategy::Patch => {
                s.push_front('~');
            },
            PatchStrategy::Minor => {
                s.push_front('^');
            },
            PatchStrategy::Major => {
                return "*".into()
            },
            _ => {}
        }
        let mut version = String::from_iter(s.iter());
        if let Some(appendix) = value.appendix {
            version.push_str(&appendix);
        }
        version
    }
}

impl TryFrom<&str> for Version {
    type Error = String;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let mut major = None; 
        let mut minor = None; 
        let mut patch = None; 
        v.find('-');
        let (v, appendix) = {
            let i = v.find('-');
            if let Some(i) = i {
                let (a, b) = v.split_at(i);
                (a, Some(b.to_string()))
            } else {
                (v, None)                
            }
        };
        let b = v.as_bytes();
        match b.first() {
            Some(f) if *f == b'~' || *f == b'^' => {
                if v.len() <= 1 {
                    return Err(format!("Failed to parse package version a version is of format x.x.x and {} was suplied", v));
                }
                for (index, v) in v[1..].split('.').enumerate() {
                    match index {
                        0 => major = Some(v.parse::<u8>().map_err(|err| err.to_string())?),
                        1 => minor = Some(v.parse::<u8>().map_err(|err| err.to_string())?),
                        2 => 
                                    patch = Some(v.parse::<u8>().map_err(|err| err.to_string())?),
                        _ => 
                        return Err(format!("Failed to parse package version a version is of format x.x.x and {} was suplied", v))
                    }
                }
                if *f == b'~' {
                Ok(Self {
                    major,
                    minor,
                    patch,
                    appendix,
                    patch_strategy: PatchStrategy::Patch,
                })
                } else {
                Ok(Self {
                    major,
                    minor,
                    patch,
                    appendix,
                    patch_strategy: PatchStrategy::Minor,
                })

                }
            }
            Some(f) if f.is_ascii_digit() => {
                let mut strategy = PatchStrategy::None;
                for (index, v) in v.split('.').enumerate() {
                    match index {
                        0 => major = Some(v.parse::<u8>().map_err(|err| err.to_string())?),
                        1 => {
                            if v == "x" {
                                strategy = PatchStrategy::Minor;
                                break;
                            }
                            minor = Some(v.parse::<u8>().map_err(|err| err.to_string())?);
                        },
                        2 => {
                            if v == "x" {
                                strategy = PatchStrategy::Patch;
                                break;
                            }
                            patch = Some(v.parse::<u8>().map_err(|err| err.to_string())?);
                        }
                        _ => 
                        return Err(format!("Failed to parse package version a version is of format x.x.x and {} was suplied", v))
                    }
                }
                Ok(Self {
                    major,
                    minor,
                    patch,
                    appendix,
                    patch_strategy: strategy,
                })
            }
            // Major releas this will in comparison allways be highest value
            Some(f) if *f == b'*' || *f == b'x' => Ok(Self {
                major,
                minor,
                patch,
                appendix,
                patch_strategy: PatchStrategy::Major,
            }),
            _ => Err("Failed parsion Version: the slice is empty".into()),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }
        if self.patch_strategy == PatchStrategy::Major {
            return Ordering::Greater;
        }
        if other.patch_strategy == PatchStrategy::Major {
            return Ordering::Less;
        }
        if self.major != other.major {
            return self.major.cmp(&other.major);
        }
        match (self.minor.cmp(&other.minor), self.patch_strategy.cmp(&other.patch_strategy)) {
            (Ordering::Less, Ordering::Less)  | 
            (Ordering::Less, Ordering::Equal) |
            (Ordering::Equal, Ordering::Less) |
            (Ordering::Greater, Ordering::Less) => Ordering::Less,
            (Ordering::Equal, Ordering::Equal) => {
                self.patch.cmp(&other.patch)
            }
            (Ordering::Less, Ordering::Greater)  | 
            (Ordering::Equal, Ordering::Greater) |
            (Ordering::Greater, Ordering::Equal) |
            (Ordering::Greater, Ordering::Greater) => Ordering::Greater
        }
    }
}

pub fn setup_mono() -> Result<(), Box<dyn Error>> {
    println!("All repos cloned: Initializing monorepo");
    let mut dependencies: HashMap<&str, (usize, Version)> = HashMap::new();
    let mut dev_dependencies: HashMap<&str, (usize, Version)> = HashMap::new();
    let dir = fs::canonicalize("../packages")?;
    let mut package_deps: Vec<serde_json::Value> = Vec::with_capacity(30);
    let mut package_dev_deps: Vec<serde_json::Value> = Vec::with_capacity(30);
    for package in fs::read_dir(dir)? {
        let mut package = package?.path();
        if !package.is_dir() {
            continue;
        }
        package.push("package.json");
        let mut v: serde_json::Value = serde_json::from_reader(fs::File::open(
                package
                )?)?;
        // Not all packages have dependencies
        let mut deps = v.as_object_mut().unwrap().remove("dependencies").unwrap_or(Value::Object(serde_json::Map::new()));
        // Add the package itself to the potential dependencies
        deps.as_object_mut().unwrap().insert(v.as_object().unwrap().get("name").unwrap().to_string(), v.as_object().unwrap().get("version").unwrap().clone());
        package_deps.push(deps);
        let mut dev_deps = v.as_object_mut().unwrap().remove("devDependencies").unwrap();
        // Add the package itself to the potential devDependencies
        dev_deps.as_object_mut().unwrap().insert(v.as_object().unwrap().get("name").unwrap().to_string(), v.as_object().unwrap().get("version").unwrap().clone()); 
        package_dev_deps.push(dev_deps);
    }
    for d in package_deps.iter() {
        for (k, ver_a) in d.as_object().unwrap() {
            let ver_a: Version = Version::try_from(ver_a.as_str().unwrap())?;
            let mut k = k.as_str();
            if k.contains('"') {
                k = &k[1..=k.len() - 2];
            }
            dependencies.entry(k).and_modify(|(n, ver_b)| {
                *n += 1;
                if ver_a > *ver_b {
                    *ver_b = ver_a.clone();
                }
            }).or_insert((1, ver_a));
        }
    }
    for d in package_dev_deps.iter() {
        for (k, ver_a) in d.as_object().unwrap() {
            let ver_a: Version = Version::try_from(ver_a.as_str().unwrap())?;
            let mut k = k.as_str();
            if k.contains('"') {
                k = &k[1..=k.len() - 2];
            }
            dev_dependencies.entry(k).and_modify(|(n, ver_b)| {
                *n += 1;
                if ver_a > *ver_b {
                    *ver_b = ver_a.clone();
                }
            }).or_insert((1, ver_a));
        }
    }
    let mut mono_package_json = default_package();
    // Sync devDependencies that commont deps are in crate root and all others share the highest
    // version so they are compatable.
    sync_deps(&mut mono_package_json, dependencies, dev_dependencies)?;
    update_webpack_config()?;
    let package_j = serde_json::to_string_pretty(&mono_package_json).unwrap();
    let mut p = std::fs::canonicalize("../")?;
    p.push("package.json");
    std::fs::write(p, package_j)?;
    println!("Monorepo setup successful");
    Ok(())
}


fn update_webpack_config() -> Result<(), Box<dyn Error>>{
    for package in fs::read_dir(fs::canonicalize("../packages")?)? {
        #[derive(Serialize, Deserialize, Debug)]
        struct Config {
            from: String,
        }
        let mut package = package?.path();
        if !package.is_dir() {
            continue;
        }
        package.push("webpack.config.js");
        let Ok(config) = fs::read_to_string(&package) else {
            continue;
        };
        let Some(begin) = config.find("patterns") else {
            continue;
        };
        let patterns = &config[begin..];
        let arr_begin = patterns.find('[').unwrap();
        let arr_end = patterns.find(']').unwrap();
        let mut config_array: Vec<Config> = json5::from_str(&patterns[arr_begin..=arr_end])?;
        for path in config_array.iter_mut() {
            path.from = format!("../.{}",path.from);
        }
        let new_webpack_config = format!("{}{}{}", &config[..begin + arr_begin], json5::to_string(&config_array)?.replace(r"\/", "/"), &config[begin + arr_end + 1..] );
        fs::write(&package, new_webpack_config)?;
    }
    Ok(())
}

fn setup_notest_script(v: &mut serde_json::Value) -> Result<(), Box<dyn Error>> {
    let scripts = v.get_mut("scripts").unwrap().as_object_mut().unwrap();
    if scripts.contains_key("build:notest") {
        return Ok(());
    }
    let build = scripts.get("build").map_or(Err("No build script found"), |r| {
        Ok(r)
    })?;
    let cmds: Vec<String> = build.as_str().unwrap().split("&&").filter_map(|c| {
        if c.contains("test") {
            return None;
        }
        let mut c = c.to_string();
        if c.contains("bundle") {
            c.push_str(":notest");
        }
        Some(c)
    }).collect();
    let build_notest = cmds.join("&&");
    scripts.insert("build:notest".into(), build_notest.into());
    // The bundle script is not mandatory
    if scripts.contains_key("bundle:notest") || !scripts.contains_key("bundle") {
        return Ok(());
    }
    let bundle = scripts.get("bundle").unwrap();
    let cmds: Vec<&'_ str> = bundle.as_str().unwrap().split("&&").filter(|c| !c.contains("test")).collect();
    let bundle_notest = cmds.join("&&");
    scripts.insert("bundle:notest".into(), bundle_notest.into());
    Ok(())
}

fn sync_deps(mono_package_json: &mut serde_json::Value, dependencies: HashMap<&str, (usize, Version)>, dev_dependencies: HashMap<&str, (usize, Version)>) -> Result<(), Box<dyn Error>>{
    let merge = MERGE_DEPS.get().unwrap();
    let mono_dev_deps = mono_package_json.get_mut("devDependencies").unwrap().as_object_mut().unwrap();
    for dep in dev_dependencies.iter().filter(|d| d.1.0 == 10) {
        mono_dev_deps.insert(dep.0.to_string(), String::from(dep.1.1.clone()).into());
    }
    for package in fs::read_dir(fs::canonicalize("../packages")?)? {
        let mut package = package?.path();
        if !package.is_dir() {
            continue;
        }
        package.push("package.json");
        let mut v: serde_json::Value = serde_json::from_reader(fs::File::open(
                &package
                )?)?;
        setup_notest_script(&mut v)?;
        if let Some(deps) = v.as_object_mut().unwrap().get_mut("dependencies") {
            let deps = deps.as_object_mut().unwrap();
            for dep in deps.iter_mut().filter(|d| d.0.starts_with("@nmshd")) {
                *dep.1 = String::from(dependencies.get(dep.0.as_str()).unwrap().1.clone()).into();
            }
            // if merge flag update the dependencies as well this is optional since it might break the
            // packages.
            if *merge {
                // Update all dependencies to be the highest version in the project.
                // This ensures all packages use common latest dependencies.
                for dep in deps.iter_mut() {
                    *dep.1 = String::from(dependencies.get(dep.0.as_str()).unwrap().1.clone()).into();
                }
            }
        }
        let dev_deps = v.as_object_mut().unwrap().get_mut("devDependencies").unwrap().as_object_mut().unwrap();
        // remove all devDependencies that are now in the package root
        dev_deps.retain(|k, _| {
            let mono_dep = dev_dependencies.get(k.as_str()).unwrap();
            mono_dep.0 != 10
        });
        // Update all devDependencies to be the highest version in the project.
        // This ensures all packages use common latest dependencies.
        for dev in dev_deps.iter_mut() {
            *dev.1 = String::from(dev_dependencies.get(dev.0.as_str()).unwrap().1.clone()).into();
        }
        fs::write(package, serde_json::to_string_pretty(&v)?)?;
    }
    Ok(())
}

fn default_package() -> serde_json::Value {
    serde_json::json!({
        "name": "monorepo",
        "version": "0.0.1",
        "description": "The Enmeshed Monorepo.",
        "homepage": "https://enmeshed.eu",
        "license": "MIT",
        "author": "j&s-soft GmbH",
        "files": [
        ],
        "scripts": {
        },
        "private": true,
        "dependencies": {
        },
        "devDependencies": {
        },
        "workspaces": {
            "packages": [
                "packages/cns-transport",
                "packages/cns-content",
                "packages/cns-consumption",
                "packages/cns-runtime",
                "packages/cns-app-runtime"
            ],
            "nohoist": [
                r#"**/@types/mocha"#,
                r#"**/@types/jest**"#
            ]
        }
     })
}

#[cfg(test)]
mod test {
    use super::Version;

    #[test]
    fn test_version_comp() {
        let ver = vec!["1.1.3",
         "1.0.3",
         "^2.3.1",
         "^1.3.3",
         "2.4.4",
         "4.3.0",
         "^0.10.3",
         "^1.3.4",
         "1.7.4",
         "2.8.5",
         "4.18.2",
         "6.0.1",
         "5.0.1",
         "1.4.1",
         "^1.4.5-lts.1",
         "0.12.0",
         "1.0.2",
         "9.3.4",
         "^0.5.3",
         "0.1.13",
         "4.6.2",
         "3.2.2",
         "3.0.4",
         "1.0.1",
         "0.3.0"];
        let mut x: Vec<Version> = ver.into_iter().map(TryFrom::try_from).map(|r| r.unwrap()).collect();
        x.sort();
        println!("{:#?}", x)
    }
}
