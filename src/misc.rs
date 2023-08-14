use misc::fs::File;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

/// Cube coordinates for hexagon tiling.
/// https://www.redblobgames.com/grids/hexagons/#conversions (use "flat" mode, not "pointy").
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct Coords {
    /// Grows towards right
    q: i16,

    /// Grows towards bottom-left
    r: i16,
    // s: i16, Grows towards top-left and is equal to [-(q+r)
}

impl Coords {
    pub fn new(q: isize, r: isize, s: isize) -> Coords {
        if q + r + s != 0 {
            panic!("Constructing an invalid Coords")
        }
        use std::convert::TryInto;
        Coords {
            q: q.try_into().unwrap(),
            r: r.try_into().unwrap(),
        }
    }

    pub fn q(&self) -> isize {
        self.q.into()
    }
    pub fn r(&self) -> isize {
        self.r.into()
    }
    pub fn s(&self) -> isize {
        -self.q() - self.r()
    }

    /// Returns the coordinates of the 6 direct neighbors, ordered clockwise starting from top.
    pub fn neighbors6(&self) -> [Coords; 6] {
        let (q, r, s) = (self.q(), self.r(), self.s());
        [
            Self::new(q + 0, r - 1, s + 1), // top
            Self::new(q + 1, r - 1, s + 0), // top-right
            Self::new(q + 1, r + 0, s - 1), // bot-right
            Self::new(q + 0, r + 1, s - 1), // bot
            Self::new(q - 1, r + 1, s + 0), // bot-left
            Self::new(q - 1, r + 0, s + 1), // top-left
        ]
    }

    /// Returns the coordinates of the 18 closest neighbors in undefined ordered
    pub fn neighbors18(&self) -> [Coords; 18] {
        let (q, r, s) = (self.q(), self.r(), self.s());
        [
            Self::new(q + 0, r - 1, s + 1),
            Self::new(q + 1, r - 1, s + 0),
            Self::new(q + 1, r + 0, s - 1),
            Self::new(q + 0, r + 1, s - 1),
            Self::new(q - 1, r + 1, s + 0),
            Self::new(q - 1, r + 0, s + 1),
            Self::new(q + 0, r - 2, s + 2),
            Self::new(q + 1, r - 2, s + 1),
            Self::new(q + 2, r - 2, s + 0),
            Self::new(q + 2, r - 1, s - 1),
            Self::new(q + 2, r + 0, s - 2),
            Self::new(q + 1, r + 1, s - 2),
            Self::new(q + 0, r + 2, s - 2),
            Self::new(q - 1, r + 2, s - 1),
            Self::new(q - 2, r + 2, s + 0),
            Self::new(q - 2, r + 1, s + 1),
            Self::new(q - 2, r + 0, s + 2),
            Self::new(q - 1, r - 1, s + 2),
        ]
    }
}

impl std::ops::Add for Coords {
    type Output = Coords;
    fn add(self, other: Coords) -> Coords {
        Coords::new(
            self.q() + other.q(),
            self.r() + other.r(),
            self.s() + other.s(),
        )
    }
}

impl std::ops::Sub for Coords {
    type Output = Coords;
    fn sub(self, other: Coords) -> Coords {
        Coords::new(
            self.q() - other.q(),
            self.r() - other.r(),
            self.s() - other.s(),
        )
    }
}

pub fn sha256<T: Serialize>(data: &T) -> String {
    let serialized_data = serde_json::to_string(data).expect("Failed to serialize the struct");
    let digest =
        crypto_hash::hex_digest(crypto_hash::Algorithm::SHA256, serialized_data.as_bytes());
    digest
}

pub fn get_url(url: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        reqwest::header::HeaderValue::from_static("MyUserAgent"),
    );
    let html = client.get(url).headers(headers).send()?.text()?;
    Ok(html)
}

pub fn get_url_with_cache(url: &String, cache_dir: &str) -> Result<String, Box<dyn Error>> {
    with_cache(url, || get_url(url), cache_dir)
}

/// Either call `compute` and cache on `Ok` or fetch from cache
/// Not thread safe
pub fn with_cache<F, T, U>(key: &T, compute: F, cache_dir: &str) -> Result<U, Box<dyn Error>>
where
    F: FnOnce() -> Result<U, Box<dyn Error>>,
    T: Serialize,
    U: Serialize + DeserializeOwned,
{
    // Decide cache filename using a crypto hash on computations' key in order to prevent hash
    // collisions
    let key = sha256(key);
    fs::create_dir_all(cache_dir)?;
    let mut path = PathBuf::from(cache_dir);
    path.push(key);
    let () = match File::open(&path) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            return Ok(serde_json::from_str(&contents).expect("Failed to deserialize the string"));
        }
        Err(error) => match error.kind() {
            ErrorKind::NotFound => (),
            _ => return Err(format!("Error while opening file: {}", error).into()),
        },
    };
    let res = compute()?;
    let json = serde_json::to_string(&res)?;
    // Write file to a tmp-named file (not thread safe), and then perform an atomic rename. It's
    // necessary in order to avoid cache corruption in case of ctrl-c during cache writing.
    let mut tmppath = PathBuf::from(cache_dir);
    tmppath.push("tmp");
    {
        let mut file = File::create(&tmppath)?;
        file.write_all(json.as_bytes())?;
    }
    fs::rename(tmppath, path)?;
    Ok(res)
}

pub fn n_choose_k(n: u64, mut k: u64) -> Option<u64> {
    if k > n {
        panic!("Bad call to n_choose_k")
    };
    if k > n - k {
        k = n - k;
    }
    let mut result: u64 = 1;
    for i in 0..k {
        let fact = n - i;
        let quot = i + 1;
        match result.checked_mul(fact) {
            None => return None,
            Some(res) => result = res / quot,
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use misc::n_choose_k;

    #[test]
    pub fn test_n_choose_k() {
        assert_eq!(n_choose_k(0, 0).unwrap(), 1);
        assert_eq!(n_choose_k(1, 0).unwrap(), 1);
        assert_eq!(n_choose_k(2, 0).unwrap(), 1);
        assert_eq!(n_choose_k(1, 1).unwrap(), 1);
        assert_eq!(n_choose_k(2, 1).unwrap(), 2);
        assert_eq!(n_choose_k(3, 1).unwrap(), 3);
        assert_eq!(n_choose_k(7, 1).unwrap(), 7);
        assert_eq!(n_choose_k(7, 2).unwrap(), 21);
        assert_eq!(n_choose_k(7, 3).unwrap(), 35);
        assert_eq!(n_choose_k(7, 4).unwrap(), 35);
        assert_eq!(n_choose_k(7, 5).unwrap(), 21);
        assert_eq!(n_choose_k(7, 6).unwrap(), 7);
        assert_eq!(n_choose_k(7, 7).unwrap(), 1);
    }
}
