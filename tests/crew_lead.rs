//! Integration tests for `specs/02-crew-lead.md` (CL-S1..S11).

// `to_vec()` on a slice (`svc.list()` returns `&[CrewLead]`) clones
// every element into a new `Vec<CrewLead>`. Used here to capture a
// snapshot before a mutation, then assert the state is UNCHANGED on
// failure — comparing slices via `assert_eq!(svc.list(), before.as_slice())`.

use passenger_resource_management::application::crew_lead_service::CrewLeadService;
use passenger_resource_management::domain::crew_lead::{CrewLead, CrewLeadId};
use passenger_resource_management::domain::errors::DomainError;

fn lead(id: &str, name: &str) -> CrewLead {
    CrewLead {
        id: CrewLeadId::from(id),
        name: name.to_owned(),
    }
}

fn three_distinct_leads() -> Vec<CrewLead> {
    vec![lead("a", "Alice"), lead("b", "Bob"), lead("c", "Cara")]
}

// -- Bootstrap (CL-S1..S4) ---------------------------------------------

#[test]
fn cl_s1_bootstrap_with_three_distinct_leads_succeeds() {
    let svc = CrewLeadService::bootstrap(three_distinct_leads()).expect("CL-S1");
    assert_eq!(svc.list().len(), 3);
}

#[test]
fn cl_s2_bootstrap_with_two_leads_fails() {
    let res = CrewLeadService::bootstrap(vec![lead("a", "Alice"), lead("b", "Bob")]);
    assert_eq!(res.err(), Some(DomainError::CrewLeadBootstrapInvalid));
}

#[test]
fn cl_s3_bootstrap_with_four_leads_fails() {
    let res = CrewLeadService::bootstrap(vec![
        lead("a", "Alice"),
        lead("b", "Bob"),
        lead("c", "Cara"),
        lead("d", "Dan"),
    ]);
    assert_eq!(res.err(), Some(DomainError::CrewLeadBootstrapInvalid));
}

#[test]
fn cl_s4_bootstrap_with_duplicate_ids_fails() {
    let res = CrewLeadService::bootstrap(vec![
        lead("a", "Alice"),
        lead("a", "Alice II"),
        lead("c", "Cara"),
    ]);
    assert_eq!(res.err(), Some(DomainError::CrewLeadBootstrapInvalid));
}

// -- Add (CL-S5) -------------------------------------------------------

#[test]
fn cl_s5_add_when_full_returns_limit_reached() {
    let mut svc = CrewLeadService::bootstrap(three_distinct_leads()).unwrap();
    let res = svc.add(lead("d", "Dan"));
    assert_eq!(res, Err(DomainError::CrewLeadLimitReached));
    assert_eq!(svc.list().len(), 3);
}

// -- Remove (CL-S6) ----------------------------------------------------

#[test]
fn cl_s6_remove_returns_minimum_breached_and_state_unchanged() {
    let mut svc = CrewLeadService::bootstrap(three_distinct_leads()).unwrap();
    let before: Vec<CrewLead> = svc.list().to_vec();
    let res = svc.remove(&CrewLeadId::from("a"));
    assert_eq!(res, Err(DomainError::CrewLeadMinimumBreached));
    assert_eq!(svc.list(), before.as_slice());
}

// -- Replace (CL-S7..S9) -----------------------------------------------

#[test]
fn cl_s7_replace_swaps_old_for_new_keeping_count_three() {
    let mut svc = CrewLeadService::bootstrap(three_distinct_leads()).unwrap();
    let new_lead = lead("d", "Dan");
    svc.replace(&CrewLeadId::from("a"), new_lead.clone())
        .expect("CL-S7");
    let ids: Vec<&CrewLeadId> = svc.list().iter().map(|l| &l.id).collect();
    assert_eq!(svc.list().len(), 3);
    assert!(ids.contains(&&CrewLeadId::from("b")));
    assert!(ids.contains(&&CrewLeadId::from("c")));
    assert!(ids.contains(&&CrewLeadId::from("d")));
    assert!(!ids.contains(&&CrewLeadId::from("a")));
}

#[test]
fn cl_s8_replace_with_unknown_old_id_returns_not_found() {
    let mut svc = CrewLeadService::bootstrap(three_distinct_leads()).unwrap();
    let before: Vec<CrewLead> = svc.list().to_vec();
    let res = svc.replace(&CrewLeadId::from("zzz"), lead("d", "Dan"));
    assert_eq!(res, Err(DomainError::CrewLeadNotFound));
    assert_eq!(svc.list(), before.as_slice());
}

#[test]
fn cl_s9_replace_with_clashing_new_id_returns_already_exists() {
    let mut svc = CrewLeadService::bootstrap(three_distinct_leads()).unwrap();
    let before: Vec<CrewLead> = svc.list().to_vec();
    // try to replace `a` with a lead whose id matches existing `b`
    let res = svc.replace(&CrewLeadId::from("a"), lead("b", "Bob II"));
    assert_eq!(res, Err(DomainError::CrewLeadAlreadyExists));
    assert_eq!(svc.list(), before.as_slice());
}

// -- Listing (CL-S11) --------------------------------------------------

#[test]
fn cl_s11_list_preserves_bootstrap_insertion_order() {
    let svc = CrewLeadService::bootstrap(three_distinct_leads()).unwrap();
    let ids: Vec<&str> = svc.list().iter().map(|l| l.id.0.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}
