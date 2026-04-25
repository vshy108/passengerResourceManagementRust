//! Serde DTOs for the HTTP adapter. These are the wire shapes —
//! domain types stay free of `serde` dependencies.

use serde::{Deserialize, Serialize};

use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::passenger::Passenger;
use crate::domain::resource::Resource;
use crate::domain::tier::Tier;
use crate::domain::usage_event::{Outcome, UsageEvent};

// ---------- tier --------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TierDto {
    Silver,
    Gold,
    Platinum,
}

impl From<Tier> for TierDto {
    fn from(t: Tier) -> Self {
        match t {
            Tier::Silver => TierDto::Silver,
            Tier::Gold => TierDto::Gold,
            Tier::Platinum => TierDto::Platinum,
        }
    }
}

impl From<TierDto> for Tier {
    fn from(t: TierDto) -> Self {
        match t {
            TierDto::Silver => Tier::Silver,
            TierDto::Gold => Tier::Gold,
            TierDto::Platinum => Tier::Platinum,
        }
    }
}

// ---------- crew lead ---------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrewLeadDto {
    pub id: String,
    pub name: String,
}

impl From<&CrewLead> for CrewLeadDto {
    fn from(c: &CrewLead) -> Self {
        Self {
            id: c.id.0.clone(),
            name: c.name.clone(),
        }
    }
}

impl From<CrewLeadDto> for CrewLead {
    fn from(d: CrewLeadDto) -> Self {
        CrewLead {
            id: CrewLeadId(d.id),
            name: d.name,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceCrewLeadReq {
    pub actor_id: String,
    pub new_lead: CrewLeadDto,
}

// ---------- passenger ---------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PassengerDto {
    pub id: String,
    pub name: String,
    pub tier: TierDto,
    pub deleted_at: Option<i64>,
}

impl From<&Passenger> for PassengerDto {
    fn from(p: &Passenger) -> Self {
        Self {
            id: p.id.0.clone(),
            name: p.name.clone(),
            tier: p.tier.into(),
            deleted_at: p.deleted_at.map(|t| t.0),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePassengerReq {
    pub actor_id: String,
    pub id: String,
    pub name: String,
    pub tier: TierDto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeTierReq {
    pub actor_id: String,
    pub tier: TierDto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorOnlyReq {
    pub actor_id: String,
}

// ---------- resource ----------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ResourceDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub min_tier: TierDto,
    pub deleted_at: Option<i64>,
}

impl From<&Resource> for ResourceDto {
    fn from(r: &Resource) -> Self {
        Self {
            id: r.id.0.clone(),
            name: r.name.clone(),
            category: r.category.clone(),
            min_tier: r.min_tier.into(),
            deleted_at: r.deleted_at.map(|t| t.0),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateResourceReq {
    pub actor_id: String,
    pub id: String,
    pub name: String,
    pub category: String,
    pub min_tier: TierDto,
}

// ---------- access ------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UseResourceReq {
    pub passenger_id: String,
    pub resource_id: String,
}

#[derive(Debug, Serialize)]
pub struct UsageEventDto {
    pub id: u64,
    pub passenger_id: String,
    pub resource_id: String,
    pub tier_at_attempt: TierDto,
    pub min_tier_at_attempt: TierDto,
    pub timestamp: i64,
    pub outcome: OutcomeDto,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum OutcomeDto {
    Allowed,
    Denied,
}

impl From<Outcome> for OutcomeDto {
    fn from(o: Outcome) -> Self {
        match o {
            Outcome::Allowed => OutcomeDto::Allowed,
            Outcome::Denied => OutcomeDto::Denied,
        }
    }
}

impl From<&UsageEvent> for UsageEventDto {
    fn from(e: &UsageEvent) -> Self {
        Self {
            id: e.id,
            passenger_id: e.passenger_id.0.clone(),
            resource_id: e.resource_id.0.clone(),
            tier_at_attempt: e.tier_at_attempt.into(),
            min_tier_at_attempt: e.min_tier_at_attempt.into(),
            timestamp: e.timestamp.0,
            outcome: e.outcome.into(),
        }
    }
}

// ---------- admin event -------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AdminEventDto {
    pub id: u64,
    pub actor_id: String,
    pub action: String,
    pub target_kind: String,
    pub target_id: String,
    pub timestamp: i64,
    pub details: Option<String>,
}

impl From<&AdminEvent> for AdminEventDto {
    fn from(e: &AdminEvent) -> Self {
        Self {
            id: e.id,
            actor_id: e.actor_id.0.clone(),
            action: admin_action_str(e.action).to_owned(),
            target_kind: target_kind_str(e.target_kind).to_owned(),
            target_id: e.target_id.clone(),
            timestamp: e.timestamp.0,
            details: e.details.clone(),
        }
    }
}

fn admin_action_str(a: AdminAction) -> &'static str {
    match a {
        AdminAction::CrewLeadBootstrapped => "CrewLeadBootstrapped",
        AdminAction::CrewLeadAdded => "CrewLeadAdded",
        AdminAction::CrewLeadRemoved => "CrewLeadRemoved",
        AdminAction::CrewLeadReplaced => "CrewLeadReplaced",
        AdminAction::PassengerCreated => "PassengerCreated",
        AdminAction::PassengerTierChanged => "PassengerTierChanged",
        AdminAction::PassengerDeleted => "PassengerDeleted",
        AdminAction::ResourceCreated => "ResourceCreated",
        AdminAction::ResourceMinTierChanged => "ResourceMinTierChanged",
        AdminAction::ResourceDeleted => "ResourceDeleted",
    }
}

fn target_kind_str(k: TargetKind) -> &'static str {
    match k {
        TargetKind::CrewLead => "CrewLead",
        TargetKind::Passenger => "Passenger",
        TargetKind::Resource => "Resource",
    }
}

// ---------- reports -----------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TierCountsDto {
    pub tier: TierDto,
    pub allowed: u64,
    pub denied: u64,
}

#[derive(Debug, Serialize)]
pub struct TopResourceDto {
    pub resource_id: String,
    pub allowed_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct TopNQuery {
    pub n: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AccessibleQuery {
    pub tier: TierDto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddCrewLeadReq {
    pub lead: CrewLeadDto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveCrewLeadReq {
    pub actor_id: String,
}

// ---------- error envelope ----------------------------------------------

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
    pub code: String,
}
