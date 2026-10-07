use crate::error::{Error, Result};
use chrono::{DateTime, Datelike, Utc};
use recollect_protocol::{ClaimContent, ClaimEligibility, ClaimRevision, FactValidity};

pub fn assertion_key(value: &str, lowercase: bool) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if lowercase {
        value.to_lowercase()
    } else {
        value
    }
}
pub(crate) fn text(value: &str, max: usize, required: bool) -> Result<()> {
    if (required && value.trim().is_empty())
        || value.chars().count() > max
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(Error::invalid(
            "Claim text is empty, too long or contains unsupported control characters.",
        ));
    }
    Ok(())
}
pub fn timestamp(value: DateTime<Utc>) -> Result<()> {
    if !(1..=9999).contains(&value.year()) || !value.timestamp_subsec_nanos().is_multiple_of(1000) {
        return Err(Error::invalid(
            "Use a timestamp in years 1–9999 with at most microsecond precision.",
        ));
    }
    Ok(())
}
pub fn validate(content: &mut ClaimContent) -> Result<()> {
    if content
        .context_role
        .as_deref()
        .is_some_and(|r| !matches!(r, "decision" | "convention" | "constraint"))
    {
        return Err(Error::invalid(
            "Choose decision, convention or constraint for a stable context role.",
        ));
    }
    content.subject = content.subject.trim().into();
    content.predicate = content.predicate.trim().into();
    content.value = content.value.trim().into();
    text(&content.subject, 256, true)?;
    text(&content.predicate, 128, true)?;
    text(&content.value, 4000, true)?;
    text(&content.rationale, 4000, false)?;
    text(&content.observation, 2000, false)?;
    if !matches!(
        content.kind.as_str(),
        "claim" | "decision" | "procedure" | "handover"
    ) || !matches!(
        content.freshness.as_str(),
        "current" | "needs_verification" | "superseded"
    ) || !matches!(
        content.operational.as_str(),
        "declared" | "implemented" | "deployed" | "verified"
    ) {
        return Err(Error::invalid(
            "Select a supported claim kind, freshness and operational assessment.",
        ));
    }
    if matches!(content.operational.as_str(), "deployed" | "verified")
        && (content.observed_at.is_none() || content.observation.trim().is_empty())
    {
        return Err(Error::invalid(
            "Deployed or verified assessments require an observation time and observed outcome.",
        ));
    }
    for at in [
        content.validity.from,
        content.validity.to,
        content.observed_at,
    ]
    .into_iter()
    .flatten()
    {
        timestamp(at)?;
    }
    let validity = &content.validity;
    let valid = match validity.kind.as_str() {
        "unknown" => {
            validity.from.is_none() && validity.to.is_none() && validity.precision == "unknown"
        }
        "point" => {
            validity.from.is_some()
                && validity.to.is_none()
                && matches!(
                    validity.precision.as_str(),
                    "second" | "minute" | "hour" | "day"
                )
        }
        "interval" => {
            (validity.from.is_some() || validity.to.is_some())
                && validity.precision == "second"
                && validity
                    .from
                    .zip(validity.to)
                    .is_none_or(|(from, to)| from < to)
        }
        _ => false,
    };
    if !valid {
        return Err(Error::invalid(
            "Use unknown validity, a point with explicit precision, or a nonempty [from,to) interval.",
        ));
    }
    let scope = &mut content.selection;
    if scope.repository_ids.len() > 100 || scope.area_ids.len() > 100 {
        return Err(Error::invalid(
            "Select at most 100 repositories and 100 areas.",
        ));
    }
    scope.repository_ids.sort_unstable();
    scope.repository_ids.dedup();
    scope.area_ids.sort_unstable();
    scope.area_ids.dedup();
    if content.supports.is_empty() || content.supports.len() > 20 {
        return Err(Error::invalid("Select 1–20 exact evidence supports."));
    }
    content.supports.sort_by_key(|s| (s.kind.clone(), s.id));
    for (index, support) in content.supports.iter().enumerate() {
        if !matches!(
            support.kind.as_str(),
            "source_version" | "repository_fact" | "manifest_revision"
        ) || support.line_from.is_some() != support.line_to.is_some()
            || support
                .line_from
                .zip(support.line_to)
                .is_some_and(|(a, b)| a < 1 || b < a || b > 1_000_000)
            || (support.kind != "source_version" && support.line_from.is_some())
            || (index > 0
                && content.supports[index - 1].kind == support.kind
                && content.supports[index - 1].id == support.id)
        {
            return Err(Error::invalid(
                "Use distinct supported evidence IDs and valid source line spans.",
            ));
        }
    }
    crate::procedures::validate(content)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeMatch {
    Matches,
    Unknown,
    Outside,
}
pub fn fact_match(validity: &FactValidity, at: Option<DateTime<Utc>>) -> TimeMatch {
    let Some(at) = at else {
        return TimeMatch::Matches;
    };
    match validity.kind.as_str() {
        "point" => {
            let Some(from) = validity.from else {
                return TimeMatch::Unknown;
            };
            let bucket = match validity.precision.as_str() {
                "second" => 1,
                "minute" => 60,
                "hour" => 3600,
                "day" => 86400,
                _ => return TimeMatch::Unknown,
            };
            if at.timestamp().div_euclid(bucket) == from.timestamp().div_euclid(bucket) {
                TimeMatch::Matches
            } else {
                TimeMatch::Outside
            }
        }
        "interval" => {
            if validity.from.is_some_and(|from| at < from) || validity.to.is_some_and(|to| at >= to)
            {
                TimeMatch::Outside
            } else if validity.from.is_none() || validity.to.is_none() {
                TimeMatch::Unknown
            } else {
                TimeMatch::Matches
            }
        }
        _ => TimeMatch::Unknown,
    }
}
pub struct Assessment {
    pub scope_valid: bool,
    pub support_available: bool,
    pub evidence_changed: bool,
    pub evidence_missing: bool,
    pub fact_time: TimeMatch,
}
pub fn eligibility(revision: &ClaimRevision, assessment: Assessment) -> ClaimEligibility {
    let mut reasons = Vec::new();
    if !assessment.scope_valid {
        reasons.push("scope_unavailable".into());
    }
    if !assessment.support_available {
        reasons.push("support_unavailable_or_reference_only".into());
    }
    if assessment.evidence_changed {
        reasons.push("supporting_evidence_changed".into());
    }
    if assessment.evidence_missing {
        reasons.push("retained_evidence_unavailable".into());
    }
    match assessment.fact_time {
        TimeMatch::Unknown => reasons.push("fact_time_unknown".into()),
        TimeMatch::Outside => reasons.push("outside_fact_time".into()),
        _ => (),
    }
    if revision.review != "accepted" {
        reasons.push(format!("review_{}", revision.review));
    }
    if revision.lifecycle != "active" {
        reasons.push("withdrawn".into());
    }
    let recorded = &revision.content.freshness;
    let freshness = if recorded == "superseded" {
        "superseded"
    } else if recorded == "needs_verification"
        || assessment.evidence_changed
        || assessment.evidence_missing
    {
        "needs_verification"
    } else {
        "current"
    };
    if freshness != "current" {
        reasons.push(freshness.into());
    }
    let investigation = assessment.scope_valid
        && assessment.fact_time != TimeMatch::Outside
        && revision.review != "rejected"
        && revision.lifecycle == "active"
        && freshness != "superseded";
    let acceptance_authority = revision.reviewer_id.is_some()
        || revision
            .acceptance_policy
            .as_ref()
            .is_some_and(|policy| !policy.trim().is_empty());
    if revision.review == "accepted" && !acceptance_authority {
        reasons.push("acceptance_authority_missing".into());
    }
    let strict_accepted = investigation
        && revision.review == "accepted"
        && acceptance_authority
        && freshness == "current"
        && assessment.support_available
        && assessment.fact_time == TimeMatch::Matches;
    let verified = revision.content.operational == "verified"
        && revision.content.observed_at.is_some()
        && !revision.content.observation.trim().is_empty();
    if !verified {
        reasons.push("operational_verification_not_recorded".into());
    }
    ClaimEligibility {
        investigation,
        strict_accepted,
        strict_operational: strict_accepted && verified,
        effective_freshness: freshness.into(),
        reasons,
        rule_ids: Vec::new(),
        conflicting_claim_ids: Vec::new(),
    }
}
