use chrono::{TimeZone, Utc};
use ggfs::structs::dir_entry::*;

#[cfg(test)]
#[test]
fn test_dir_entry() {
    let data = hex_literal::hex!(
        "4b 45 52 4e 45 4c 20 20 45 4c 46 20 00 00 0f be
         d0 50 d0 50 00 00 0f be d0 50 02 00 f0 e4 0e 00"
    );

    let res = DirEntry::parse(&data).unwrap();

    assert_eq!(&res.filename.name, b"KERNEL  ");
    assert_eq!(&res.filename.ext, b"ELF");
    assert_eq!(res.attributes, Attributes::ARCHIVE);
    assert_eq!(res.cluster, Cluster(2));
    assert_eq!(res.size, 0xee4f0);
    assert_eq!(
        res.created_time,
        Utc.with_ymd_and_hms(2020, 6, 16, 23, 48, 30).unwrap()
    );
    assert_eq!(
        res.moditified_time,
        Utc.with_ymd_and_hms(2020, 6, 16, 23, 48, 30).unwrap()
    );
    assert_eq!(
        res.accessed_time,
        Utc.with_ymd_and_hms(2020, 6, 16, 0, 0, 0).unwrap()
    );

    println!("{:#?}", res);
}
