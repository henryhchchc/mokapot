use std::{fs, io, path::Path};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        hash = (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Debug, Clone, Copy)]
pub struct Shard {
    pub index: u64,
    pub count: u64,
}

impl Shard {
    pub fn new(index: Option<&str>, count: Option<&str>) -> io::Result<Self> {
        let parse = |value: &str| {
            value.parse::<u64>().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid shard value {value:?}: {error}"),
                )
            })
        };
        let index = parse(index.unwrap_or("0"))?;
        let count = parse(count.unwrap_or("16"))?;
        if count == 0 || index >= count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "shard count must be positive and index must be less than count",
            ));
        }
        Ok(Self { index, count })
    }

    pub fn contains(self, relative_path: &str) -> bool {
        hash_bytes(FNV_OFFSET, relative_path.as_bytes()) % self.count == self.index
    }
}

pub fn relative_name(root: &Path, path: &Path) -> io::Result<String> {
    let relative = path.strip_prefix(root).map_err(io::Error::other)?;
    relative
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("non-UTF-8 class path: {}", path.display()),
                    )
                })
        })
        .collect::<io::Result<Vec<_>>>()
        .map(|components| components.join("/"))
}

pub struct ClassInput {
    pub name: String,
    pub bytes: Vec<u8>,
}

pub struct Corpus {
    pub classes: Vec<ClassInput>,
    pub byte_count: usize,
    pub fingerprint: u64,
}

impl Corpus {
    pub fn load(root: &Path, shard: Shard) -> io::Result<Self> {
        if !fs::metadata(root)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("JDK_CLASSES must name a directory: {}", root.display()),
            ));
        }
        let mut paths = Vec::new();
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry.map_err(io::Error::other)?;
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "class")
            {
                let name = relative_name(root, entry.path())?;
                if shard.contains(&name) {
                    paths.push((name, entry.into_path()));
                }
            }
        }
        paths.sort_by(|(left, _), (right, _)| left.cmp(right));
        if paths.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "shard {} of {} contains no classes under {}",
                    shard.index,
                    shard.count,
                    root.display()
                ),
            ));
        }
        let mut classes = Vec::with_capacity(paths.len());
        let mut byte_count = 0;
        let mut fingerprint = FNV_OFFSET;
        for (name, path) in paths {
            let bytes = fs::read(&path).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("cannot read {}: {error}", path.display()),
                )
            })?;
            byte_count += bytes.len();
            // Length prefixes make path/content boundaries unambiguous.
            fingerprint = hash_bytes(fingerprint, &(name.len() as u64).to_le_bytes());
            fingerprint = hash_bytes(fingerprint, name.as_bytes());
            fingerprint = hash_bytes(fingerprint, &(bytes.len() as u64).to_le_bytes());
            fingerprint = hash_bytes(fingerprint, &bytes);
            classes.push(ClassInput { name, bytes });
        }
        Ok(Self {
            classes,
            byte_count,
            fingerprint,
        })
    }
}
