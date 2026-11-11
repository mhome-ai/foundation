//! Outcome rules owned by the Space mutation contracts, before presentation.
use crate::FacadeResponse;
use serde_json::Value;

pub(super) fn outcome(target: &str, data: &Value) -> Option<FacadeResponse> {
    let fulfilled = match target {
        "/app/space/member/update" => return Some(changes(data)),
        "/app/space/member/invite" => &["invited", "alreadyPending", "alreadyMembers"][..],
        "/app/space/member/remove" => &["removed", "notMembership"][..],
        "/app/space/location/create" => return location(data, "Created", "NotCreated"),
        "/app/space/location/update" => return location(data, "Updated", "NotUpdated"),
        "/app/space/location/delete" => return location(data, "Deleted", "NotDeleted"),
        _ => return None,
    };
    // A prepared preview is control data; never interpret its planned rows as effects.
    if data["action"] == "DryRun" {
        return Some(FacadeResponse::success(data.clone()));
    }
    let Some(failed) = data.get("failed").and_then(Value::as_array) else {
        return Some(FacadeResponse::error(
            "INVALID_RESPONSE",
            "Expected Space member outcome",
        ));
    };
    if fulfilled
        .iter()
        .any(|key| !data.get(*key).is_some_and(Value::is_array))
    {
        return Some(FacadeResponse::error(
            "INVALID_RESPONSE",
            "Expected Space member outcome",
        ));
    }
    Some(if failed.is_empty() {
        FacadeResponse::success(data.clone())
    } else if fulfilled
        .iter()
        .any(|key| !data[*key].as_array().unwrap().is_empty())
    {
        FacadeResponse::partial(data.clone())
    } else {
        FacadeResponse::error(
            "OPERATION_FAILED",
            "No requested Space member operation succeeded",
        )
        .with_details(data.clone())
    })
}

fn changes(data: &Value) -> FacadeResponse {
    if data["action"] == "DryRun" {
        return FacadeResponse::success(data.clone());
    }
    let groups_valid = |group: &Value, fields: &[&str]| {
        fields
            .iter()
            .all(|key| group.get(*key).is_some_and(Value::is_array))
    };
    if !data.get("problems").is_some_and(Value::is_array)
        || !groups_valid(&data["removal"], &["removed", "notMembership", "failed"])
        || !(groups_valid(
            &data["invitation"],
            &["invited", "alreadyPending", "alreadyMembers", "failed"],
        ) || data.get("invitation").is_some_and(Value::is_null)
            && !data["problems"].as_array().unwrap().is_empty())
    {
        return FacadeResponse::error("INVALID_RESPONSE", "Expected Space member changes outcome");
    }
    let fulfilled = [
        data.pointer("/removal/removed"),
        data.pointer("/removal/notMembership"),
        data.pointer("/invitation/invited"),
        data.pointer("/invitation/alreadyPending"),
        data.pointer("/invitation/alreadyMembers"),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_array)
    .map(Vec::len)
    .sum::<usize>();
    let failures = [
        data.pointer("/removal/failed"),
        data.pointer("/invitation/failed"),
        data.get("problems"),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_array)
    .map(Vec::len)
    .sum::<usize>();
    if failures == 0 {
        FacadeResponse::success(data.clone())
    } else if fulfilled > 0 {
        FacadeResponse::partial(data.clone())
    } else {
        FacadeResponse::error("OPERATION_FAILED", "Space member changes did not complete")
            .with_details(data.clone())
    }
}

fn location(data: &Value, success: &str, failure: &str) -> Option<FacadeResponse> {
    Some(match data.get("action").and_then(Value::as_str) {
        Some("DryRun") => FacadeResponse::success(data.clone()),
        Some(action) if action == success => FacadeResponse::success(data.clone()),
        Some(action) if action == failure => FacadeResponse::error(
            "OPERATION_FAILED",
            "The Space Location write did not succeed",
        )
        .with_details(data.clone()),
        _ => FacadeResponse::error("INVALID_RESPONSE", "Expected Space Location write outcome"),
    })
}
