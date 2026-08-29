#![cfg(test)]

use recite_ui::{Client, ClientSpec, ResourceId, ResourceSpec, UiArgType, UiContract};

#[test]
fn contract_rejects_arguments_present_in_only_one_selector_variant() {
    let contract = UiContract::new(
        vec![
            ResourceSpec::new(ResourceId::new("hello").expect("id"))
                .argument("kind", UiArgType::String)
                .argument("detail", UiArgType::String)
                .client(Client::Cli),
        ],
        vec![ClientSpec::new(Client::Cli, "CLI", true)],
    );
    let error = contract
        .validate("hello = { $kind ->\n   [one] One { $detail }\n  *[other] Other\n}\n")
        .expect_err("selector branches must expose the same arguments");
    assert!(error.issues.iter().any(|issue| matches!(
        issue,
        recite_ui::ContractIssue::SelectorArgumentMismatch { id, name }
            if id == "hello" && name == "detail"
    )));
}

#[test]
fn contract_rejects_branch_local_arguments_in_nested_selectors() {
    let contract = UiContract::new(
        vec![
            ResourceSpec::new(ResourceId::new("hello").expect("id"))
                .argument("kind", UiArgType::String)
                .argument("nested", UiArgType::String)
                .argument("detail", UiArgType::String)
                .client(Client::Cli),
        ],
        vec![ClientSpec::new(Client::Cli, "CLI", true)],
    );
    let error = contract
        .validate(
            "hello = { $kind ->\n   [one] { $nested ->\n      [yes] Yes { $detail }\n     *[no] No\n   }\n  *[other] Other { $nested }\n}\n",
        )
        .expect_err("nested selector branches must expose the same arguments");
    assert!(error.issues.iter().any(|issue| matches!(
        issue,
        recite_ui::ContractIssue::SelectorArgumentMismatch { id, name }
            if id == "hello" && name == "detail"
    )));
}
