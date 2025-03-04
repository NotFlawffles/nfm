use std::io::Result;

use nfm::NFM;

mod action;
mod mode;
mod nfm;
mod window;

fn main() -> Result<()> {
    let mut nfm = NFM::new();
    nfm.run()
}
