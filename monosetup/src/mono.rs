use std::{cmp::Ordering, error::Error, fs, collections::{HashMap, VecDeque}};

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
        for mut v in [value.patch, value.minor, value.major].into_iter().filter_map(|v| {
            v
        }) {
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
        String::from_iter(s.iter())
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
                (&v[..], None)                
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
    let mut dependencies: HashMap<&str, (usize, Version)> = HashMap::new();
    let dir = fs::canonicalize("../packages")?;
    let mut package_deps: Vec<serde_json::Value> = Vec::with_capacity(30);
    for package in fs::read_dir(dir)? {
        let mut package = package?.path();
        package.push("package.json");
        let mut v: serde_json::Value = serde_json::from_reader(fs::File::open(
                package
                )?)?;
        let deps = v.as_object_mut().unwrap().remove("dependencies").unwrap();
        package_deps.push(deps);
    }
    for d in package_deps.iter() {
        for (k, ver_a) in d.as_object().unwrap() {
            let ver_a: Version = Version::try_from(ver_a.as_str().unwrap())?;
            dependencies.entry(k.as_str()).and_modify(|(n, ver_b)| {
                *n += 1;
                if ver_a > *ver_b {
                    *ver_b = ver_a.clone();
                }
            }).or_insert((1, ver_a));
        }
    }
    let mut mono_package_json = default_package();
    let mono_deps = mono_package_json.get_mut("dependencies").unwrap().as_object_mut().unwrap();
    for dep in dependencies.iter().filter(|(_, (c, _))| *c >= 1) {
        mono_deps.insert(dep.0.to_string(), String::from(dep.1.1.clone()).into());
    }
    println!("{}", serde_json::to_string_pretty(&mono_package_json).unwrap());
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
        "dependencies": {
        },
        "devDependencies": {
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
