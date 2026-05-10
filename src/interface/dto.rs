//! Serde DTOs for the HTTP adapter. These are the wire shapes —
//! domain types stay free of `serde` dependencies.

// `serde` (SERialize / DEserialize) is THE Rust serialisation framework.
// Crate-level derives turn structs into JSON / YAML / etc. with one line.
//   - `Serialize`   -> can be encoded to JSON (responses).
//   - `Deserialize` -> can be decoded from JSON (requests).
// `utoipa::ToSchema` generates an OpenAPI schema for the type.
use serde::{Deserialize, Serialize};

use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::passenger::Passenger;
use crate::domain::resource::Resource;
use crate::domain::tier::Tier;
use crate::domain::usage_event::{Outcome, UsageEvent};

// Why TWO sets of types (domain `Tier` vs wire `TierDto`)?
// - Keeps domain free of serde / utoipa dependencies (clean architecture).
// - Lets the wire format evolve independently from internal types.
// - Boundary is the place to enforce strictness like `deny_unknown_fields`.
// Conversions go through `From` impls below — symmetric and explicit.

// ---------- tier --------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TierDto {
    Silver,
    Gold,
    Diamond,
    Platinum,
}

// `From<A> for B` reads as "given an A, produce a B".
// Implementing it also gives `a.into()` for free in either direction
// where types are unambiguous.
impl From<Tier> for TierDto {
    fn from(t: Tier) -> Self {
        match t {
            Tier::Silver => TierDto::Silver,
            Tier::Gold => TierDto::Gold,
            Tier::Diamond => TierDto::Diamond,
            Tier::Platinum => TierDto::Platinum,
        }
    }
}

impl From<TierDto> for Tier {
    fn from(t: TierDto) -> Self {
        match t {
            TierDto::Silver => Tier::Silver,
            TierDto::Gold => Tier::Gold,
            TierDto::Diamond => Tier::Diamond,
            TierDto::Platinum => Tier::Platinum,
        }
    }
}

// ---------- crew lead ---------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
// `#[serde(deny_unknown_fields)]` rejects request bodies with extra
// keys instead of silently ignoring them. Strong boundary validation
// per AGENTS.md §7 — catches typos and malicious payloads early.
#[serde(deny_unknown_fields)]
pub struct CrewLeadDto {
    pub id: String,
    pub name: String,
}

impl CrewLeadDto {
    /// Validate string-field lengths at the interface boundary.
    /// Rejects IDs / names that exceed 255 chars to prevent oversized payloads
    /// from reaching the domain layer (OWASP A03 — injection / resource abuse).
    ///
    /// # Errors
    ///
    /// Returns a static error message string if any field is empty or exceeds 255 characters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty() {
            return Err("id must not be empty");
        }
        if self.id.len() > 255 {
            return Err("id must be 255 characters or fewer");
        }
        if self.name.is_empty() {
            return Err("name must not be empty");
        }
        if self.name.len() > 255 {
            return Err("name must be 255 characters or fewer");
        }
        Ok(())
    }
}

// `From<&CrewLead>` (borrow): converts WITHOUT consuming the source.
// Useful when serialising a response from a service-owned value.
impl From<&CrewLead> for CrewLeadDto {
    fn from(c: &CrewLead) -> Self {
        Self {
            // `.0` reaches inside the newtype to its inner String.
            id: c.id.0.clone(),
            name: c.name.clone(),
        }
    }
}

// `From<CrewLeadDto>` (by value): consumes the DTO so we can MOVE its
// String fields into the domain type — zero clones.
impl From<CrewLeadDto> for CrewLead {
    fn from(d: CrewLeadDto) -> Self {
        CrewLead {
            id: CrewLeadId(d.id),
            name: d.name,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplaceCrewLeadReq {
    pub new_lead: CrewLeadDto,
}

// ---------- passenger ---------------------------------------------------

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
            // `.into()` calls `From<Tier> for TierDto` defined above —
            // type inferred from the field's declared type.
            tier: p.tier.into(),
            // `Option::map` transforms `Some(x)` -> `Some(f(x))`,
            // leaves `None` unchanged. Here we extract the inner i64
            // from `Timestamp` for the wire form.
            deleted_at: p.deleted_at.map(|t| t.0),
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePassengerReq {
    pub id: String,
    pub name: String,
    pub tier: TierDto,
}

impl CreatePassengerReq {
    /// # Errors
    ///
    /// Returns a static error message string if any field is empty or exceeds 255 characters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty() {
            return Err("id must not be empty");
        }
        if self.id.len() > 255 {
            return Err("id must be 255 characters or fewer");
        }
        if self.name.is_empty() {
            return Err("name must not be empty");
        }
        if self.name.len() > 255 {
            return Err("name must be 255 characters or fewer");
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeTierReq {
    pub tier: TierDto,
}

// ---------- resource ----------------------------------------------------

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateResourceReq {
    pub id: String,
    pub name: String,
    pub category: String,
    pub min_tier: TierDto,
}

impl CreateResourceReq {
    /// # Errors
    ///
    /// Returns a static error message string if any field is empty or exceeds 255 characters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty() {
            return Err("id must not be empty");
        }
        if self.id.len() > 255 {
            return Err("id must be 255 characters or fewer");
        }
        if self.name.is_empty() {
            return Err("name must not be empty");
        }
        if self.name.len() > 255 {
            return Err("name must be 255 characters or fewer");
        }
        if self.category.is_empty() {
            return Err("category must not be empty");
        }
        if self.category.len() > 255 {
            return Err("category must be 255 characters or fewer");
        }
        Ok(())
    }
}

// ---------- access ------------------------------------------------------

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UseResourceReq {
    pub resource_id: String,
}

impl UseResourceReq {
    /// # Errors
    ///
    /// Returns a static error message string if `resource_id` is empty or exceeds 255 characters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.resource_id.is_empty() {
            return Err("resource_id must not be empty");
        }
        if self.resource_id.len() > 255 {
            return Err("resource_id must be 255 characters or fewer");
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UsageEventDto {
    pub id: String,
    pub passenger_id: String,
    pub resource_id: String,
    pub tier_at_attempt: TierDto,
    pub min_tier_at_attempt: TierDto,
    pub timestamp: i64,
    pub outcome: OutcomeDto,
}

#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
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
            id: e.id.clone(),
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminEventDto {
    pub id: String,
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
            id: e.id.clone(),
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
    // `&'static str` = a string slice that lives for the entire program
    // lifetime. String *literals* like "Hello" are `&'static str` —
    // they're baked into the binary, no allocation. Cheaper than
    // returning `String` here since we never need to mutate them.
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TierCountsDto {
    pub tier: TierDto,
    pub allowed: u64,
    pub denied: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TopResourceDto {
    pub resource_id: String,
    pub allowed_count: u64,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TopNQuery {
    pub n: Option<usize>,
}

impl TopNQuery {
    /// Returns `n`, defaulting to 5 and capped at 1 000 to prevent `DoS` via
    /// unbounded result-set requests at the boundary (OWASP A04).
    #[must_use]
    pub fn n(&self) -> usize {
        self.n.unwrap_or(5).min(1_000)
    }
}

/// Offset-based pagination query parameters.
/// `offset` defaults to 0; `limit` defaults to 100 (max 1000).
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl PaginationQuery {
    #[must_use]
    pub fn offset(&self) -> usize {
        // FIX: cap offset so unbounded skip values cannot be used to enumerate
        // beyond a sane range; mirrors the limit cap for consistency (OWASP A04).
        self.offset.unwrap_or(0).min(1_000_000)
    }
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(100).min(1000)
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AccessibleQuery {
    pub tier: TierDto,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddCrewLeadReq {
    pub lead: CrewLeadDto,
}

// ---------- error envelope ----------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub error: String,
    pub code: String,
}

// ---------- health ready ------------------------------------------------

/// Response body for `GET /health/ready`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthReadyDto {
    /// Always `"ready"` on a 200 response.
    pub status: String,
    /// Semver of the running binary, e.g. `"1.0.0"`.
    pub version: String,
    pub crew_leads: usize,
    pub passengers_active: usize,
    pub resources_active: usize,
    pub usage_events: usize,
    pub admin_events: usize,
}
