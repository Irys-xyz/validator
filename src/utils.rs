use std::{
    fs::{self, File},
    io::{Error, Read},
};

pub fn get_file_as_byte_vector(filename: &str) -> Result<Vec<u8>, Error> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    let res = f.read(&mut buffer);
    if let Err(err) = res {
        Err(err)
    } else {
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::get_file_as_byte_vector;

    #[test]
    fn read_file_should_read_correctly() {
        let file_byte_size: usize = 2192;
        let file_path = "./bundles/test_bundle";
        let read = get_file_as_byte_vector(file_path);
        assert_eq!(read.unwrap().len(), file_byte_size)
    }
}
