use locardx_verification::StreamHasher;

#[test]
fn test_verification_crate_exports() {
    let _ = std::mem::size_of::<StreamHasher>();
}
