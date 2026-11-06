use app_facade_api::authorization;

#[test]
fn groups_use_only_facade_routes() {
    for target in [
        "/integration/group/get",
        "/integration/group/save",
        "/integration/group/delete",
    ] {
        assert!(authorization::required(target).is_none(), "{target}");
    }
    for target in ["/app/group/list", "/app/group/detail/get"] {
        assert_eq!(
            authorization::required(target),
            Some(authorization::OperationPolicy::SpaceRead)
        );
    }
    for target in [
        "/app/group/create",
        "/app/group/rename",
        "/app/group/members/update",
        "/app/group/delete",
    ] {
        assert_eq!(
            authorization::required(target),
            Some(authorization::OperationPolicy::SpaceWrite)
        );
    }
}
