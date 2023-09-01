use std::{cmp::Ordering, error::Error, fs, collections::HashMap};

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
    let mut dependencies: HashMap<String, (usize, Version)> = HashMap::new();
    let dir = fs::canonicalize("../packages")?;
    for package in fs::read_dir(dir)? {
        let mut package = package?.path();
        package.push("package.json");
        let mut dep: serde_json::Value = serde_json::from_reader(fs::File::open(
                package
                )?)?;
        let dep = dep.get_mut("dependencies").unwrap();
        for (k, ver_a) in dep.as_object().unwrap() {
            let ver_a: Version = Version::try_from(ver_a.as_str().unwrap())?;
            let k = k.clone();
            dependencies.entry(k).and_modify(|(n, ver_b)| {
                *n += 1;
                if ver_a > *ver_b {
                    *ver_b = ver_a.clone();
                }
            }).or_insert((1, ver_a));
        }
    }
    Ok(())
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
