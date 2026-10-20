use super::*;
use crate::declarations::session::tests::session;

#[test]
fn save_captures_output_by_identity_and_retires_it_with_a_single_use_receipt() {
    let (_directory, mut session) = session().unwrap();
    let previous = session.schema_output.clone();
    let captured = session.save_request().unwrap();
    assert!(Arc::ptr_eq(&previous, &captured.expected_output));
    let mut request = captured.prepare().unwrap();
    request.write_source().unwrap();
    session
        .accept_saved_source(&request)
        .unwrap()
        .wait()
        .unwrap();
    request.write_output().unwrap();
    session.accept_saved_output(&mut request).unwrap();
    assert!(Arc::ptr_eq(
        &previous,
        request.retired_output.as_ref().unwrap()
    ));
    let published = session.schema_output.clone();
    assert!(session.accept_saved_output(&mut request).is_err());
    assert!(Arc::ptr_eq(&published, &session.schema_output));
}
