//! Embedded notices travel with the standalone compiler and editor binaries.
/// License and source notices for LAML's bundled visual resources.
pub const TEXT: &str = concat!(
    "LAML compiler (MIT)\n",
    include_str!("../LICENSE"),
    "\nInter\n",
    include_str!("../assets/INTER-LICENSE.txt"),
    "\nGeist\n",
    include_str!("../assets/GEIST-LICENSE.txt"),
    "\nGeist source\n",
    include_str!("../assets/GEIST-SOURCE.txt"),
    "\nPhosphor\n",
    include_str!("../assets/phosphor/LICENSE"),
    "\nMatching LAML source: https://github.com/farion/laml/releases\n",
);
