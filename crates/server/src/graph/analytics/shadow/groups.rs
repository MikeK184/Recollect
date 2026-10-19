//! Scalar output construction for the five frozen experimental arms. This does
//! not cluster, infer semantic identities, publish topics or qualify a recipe.
use super::*;

type Term = (String, String);

fn terms(sources: &[Source]) -> BTreeMap<Term, BTreeSet<Uuid>> {
    let mut terms = BTreeMap::<Term, BTreeSet<Uuid>>::new();
    for source in sources {
        if let Some(payload) = &source.payload {
            for mention in &payload.mentions {
                if matches!(
                    mention.kind,
                    mapping::EntityKind::Technology | mapping::EntityKind::Concept
                ) {
                    terms
                        .entry((
                            mention.kind.as_str().into(),
                            mention.kind.normalized_label(&mention.span.quote),
                        ))
                        .or_default()
                        .insert(source.version);
                }
            }
        }
    }
    terms
}

fn span_label(
    sources: &[Source],
    members: &BTreeSet<Uuid>,
    term: &Term,
) -> Result<(String, LabelProvenance)> {
    sources
        .iter()
        .filter(|s| members.contains(&s.version))
        .flat_map(|s| {
            s.payload
                .iter()
                .flat_map(|p| p.mentions.iter())
                .filter_map(move |m| {
                    ((m.kind.as_str(), m.kind.normalized_label(&m.span.quote))
                        == (term.0.as_str(), term.1.clone()))
                        .then_some((
                            s.version,
                            m.span.byte_start,
                            m.span.byte_end,
                            m.span.quote.clone(),
                        ))
                })
        })
        .min()
        .map(|(version, start, end, quote)| {
            (
                quote.clone(),
                LabelProvenance {
                    kind: "exact_entity_mention".into(),
                    source_version_id: version,
                    byte_start: Some(start),
                    byte_end: Some(end),
                    quote: Some(quote),
                    field_kind: None,
                    field_id: None,
                },
            )
        })
        .ok_or_else(invalid)
}

fn community_label(
    sources: &[Source],
    members: &BTreeSet<Uuid>,
    eligible: &BTreeMap<Term, BTreeSet<Uuid>>,
    use_terms: bool,
) -> Result<(String, LabelProvenance)> {
    // The same exact Brain-local technology/concept identities feed affinity.
    // Count distinct sources, then break ties by canonical term key; repeated
    // occurrences and source order cannot inflate or change the chosen label.
    let term = use_terms
        .then(|| {
            eligible
                .iter()
                .map(|(term, versions)| {
                    mapping::topic_labels::term_rank(versions.intersection(members).count(), term)
                })
                .filter(|(count, _)| count.0 > 0)
                .min()
        })
        .flatten();
    if let Some((_, term)) = term {
        return span_label(sources, members, term);
    }
    let source = sources
        .iter()
        .filter(|s| members.contains(&s.version))
        .min_by_key(|s| s.version)
        .ok_or_else(invalid)?;
    Ok((
        source.label.clone(),
        LabelProvenance {
            kind: "canonical_title".into(),
            source_version_id: source.version,
            byte_start: None,
            byte_end: None,
            quote: None,
            field_kind: None,
            field_id: None,
        },
    ))
}

pub(super) fn build(
    report: Uuid,
    brain: Uuid,
    p: &Prepared,
    native_groups: &BTreeMap<Uuid, i64>,
    actual_gds_version: Option<String>,
) -> Result<ShadowResult> {
    let by_id: BTreeMap<_, _> = p.sources.iter().map(|s| (s.id, s)).collect();
    let versions: BTreeSet<_> = p.sources.iter().map(|s| s.version).collect();
    if by_id.len() != p.sources.len()
        || versions.len() != p.sources.len()
        || native_groups.keys().any(|id| !by_id.contains_key(id))
        || (!p.mode.native() && !native_groups.is_empty())
        || (!native_groups.is_empty() && actual_gds_version.as_deref().is_none_or(str::is_empty))
        || p.sources.iter().any(|s| {
            s.payload
                .as_ref()
                .is_some_and(|payload| payload.source_version_id != s.version)
                || s.payload.is_some() && (s.candidate.is_none() || s.request.is_none())
        })
    {
        return Err(invalid());
    }
    let eligible = terms(&p.sources);
    let mut groups = vec![];
    if p.mode.native() {
        let mut communities = BTreeMap::<i64, BTreeSet<Uuid>>::new();
        for (id, community) in native_groups {
            communities
                .entry(*community)
                .or_default()
                .insert(by_id[id].version);
        }
        // Report-local IDs depend on sorted actual contributor sets, never on
        // unstable GDS community numbers or incoming source/assignment order.
        let communities: BTreeSet<_> = communities.into_values().collect();
        for members in communities {
            // Native term vertices can truthfully back one-source communities.
            // Vector-only isolated sources must not become fabricated topics.
            if p.mode == Mode::EligibleVector && members.len() < 2 {
                continue;
            }
            let (label, label_provenance) =
                community_label(&p.sources, &members, &eligible, p.mode.terms())?;
            groups.push(Group {
                id: format!("shadow:{report}:community:{}", groups.len()),
                kind: "native_community".into(),
                label,
                label_provenance,
                source_versions: members.into_iter().collect(),
            });
        }
    }
    if p.mode.terms() {
        for (index, (term, members)) in eligible.iter().enumerate() {
            let (label, label_provenance) = span_label(&p.sources, members, term)?;
            groups.push(Group {
                id: format!("shadow:{report}:term:{index}"),
                kind: "entity_facet".into(),
                label,
                label_provenance,
                source_versions: members.iter().copied().collect(),
            });
        }
    }
    if p.mode == Mode::Metadata {
        let mut metadata = p.metadata_groups.clone();
        metadata.sort_by(|a, b| {
            (
                &a.label_provenance.field_kind,
                a.label_provenance.field_id,
                &a.source_versions,
            )
                .cmp(&(
                    &b.label_provenance.field_kind,
                    b.label_provenance.field_id,
                    &b.source_versions,
                ))
        });
        let mut fields = BTreeSet::new();
        for (index, mut group) in metadata.into_iter().enumerate() {
            group.source_versions.sort_unstable();
            let members: BTreeSet<_> = group.source_versions.iter().copied().collect();
            let provenance = &group.label_provenance;
            if members.len() != group.source_versions.len()
                || members.is_empty()
                || !members.is_subset(&versions)
                || !members.contains(&provenance.source_version_id)
                || provenance.kind != "canonical_scope"
                || !matches!(
                    provenance.field_kind.as_deref(),
                    Some("repository" | "environment" | "area")
                )
                || provenance.field_id.is_none()
                || provenance.byte_start.is_some()
                || provenance.byte_end.is_some()
                || provenance.quote.is_some()
                || !fields.insert((provenance.field_kind.clone(), provenance.field_id))
            {
                return Err(invalid());
            }
            group.id = format!("shadow:{report}:metadata:{index}");
            group.kind = "canonical_scope".into();
            groups.push(group);
        }
    }
    if groups
        .iter()
        .any(|g| g.label.is_empty() || g.label.len() > 1024)
    {
        return Err(invalid());
    }
    let mut sources: Vec<_> = p
        .sources
        .iter()
        .map(|s| {
            let memberships: Vec<_> = groups
                .iter()
                .filter(|g| g.source_versions.contains(&s.version))
                .map(|g| g.id.clone())
                .collect();
            SourceOutcome {
                source_version_id: s.version,
                candidate_id: s.candidate,
                request_id: s.request,
                state: if memberships.is_empty() {
                    "unassigned"
                } else {
                    "assigned"
                }
                .into(),
                ner_feature_available: s.payload.is_some(),
                vector_feature_available: p.vectors.available.contains(&s.version),
                groups: memberships,
            }
        })
        .collect();
    sources.sort_by_key(|s| s.source_version_id);
    let mut vector_entry_ids: Vec<_> = p.vectors.entries.iter().map(|e| e.entry).collect();
    vector_entry_ids.sort_unstable();
    vector_entry_ids.dedup();
    Ok(ShadowResult {
        report_id: report,
        brain_id: brain,
        mode: p.mode,
        parameters: p.parameters,
        gds_version: actual_gds_version,
        expires_at: p.deadline,
        sources,
        groups,
        vector_profile_id: p.vectors.profile,
        vector_entry_ids,
        vector_coverage: p.vectors.coverage.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapping::{EntityKind, MentionCandidate, MentionSpan, SourceCandidates};

    fn source(n: u128, kind: EntityKind, label: &str) -> Source {
        let version = Uuid::from_u128(n);
        Source {
            id: Uuid::from_u128(n + 100),
            source_id: Uuid::from_u128(n + 200),
            version,
            candidate: Some(Uuid::from_u128(n + 300)),
            context: serde_json::json!({}),
            label: format!("Source {n}"),
            request: Some(Uuid::from_u128(n + 400)),
            payload: Some(SourceCandidates {
                schema: mapping::SOURCE_SCHEMA.into(),
                adapter_revision: "test".into(),
                source_version_id: version,
                language: "en".into(),
                mentions: vec![MentionCandidate {
                    key: "m1".into(),
                    kind,
                    span: MentionSpan {
                        byte_start: 2,
                        byte_end: 2 + label.len(),
                        quote: label.into(),
                    },
                    confidence: 1.0,
                }],
                aliases: vec![],
                relations: vec![],
            }),
        }
    }
    fn prepared(mode: Mode) -> Prepared {
        Prepared {
            epoch: 1,
            policy: Uuid::nil(),
            sources: vec![
                source(1, EntityKind::Technology, "Rust"),
                source(2, EntityKind::Technology, "rust"),
                source(3, EntityKind::Person, "Alice"),
            ],
            vectors: RetainedFeatures {
                profile: None,
                entries: vec![],
                available: BTreeSet::new(),
                coverage: "unavailable".into(),
                budget: false,
            },
            projection: affinity::prepare(&[], &[]).unwrap(),
            mode,
            parameters: HybridParameters::default(),
            gds_version: "test".into(),
            deadline: Utc::now() + chrono::Duration::days(1),
            metadata_groups: vec![],
        }
    }
    fn output(p: &Prepared, groups: &[(u128, i64)]) -> Result<ShadowResult> {
        build(
            Uuid::nil(),
            Uuid::nil(),
            p,
            &groups
                .iter()
                .map(|(id, g)| (Uuid::from_u128(*id), *g))
                .collect(),
            p.mode.native().then(|| "test-native".into()),
        )
    }
    #[test]
    fn native_and_facet_identity_labels_survive_order_and_gds_renumbering() {
        let mut p = prepared(Mode::NativeHybrid);
        let a = output(&p, &[(101, 7), (102, 7), (103, 9)]).unwrap_or_else(|e| panic!("{}", e.1));
        p.sources.reverse();
        let b = output(&p, &[(101, 42), (102, 42), (103, 1)]).unwrap_or_else(|e| panic!("{}", e.1));
        assert_eq!(a, b);
        assert_eq!(a.groups.len(), 3); // two actual native communities, one facet
        assert_eq!(a.groups[0].label, "Rust");
        assert_eq!(a.groups[0].label_provenance.byte_start, Some(2));
        assert_eq!(a.groups[1].label, "Source 3");
        assert_eq!(a.groups[1].label_provenance.kind, "canonical_title");
        assert!(a.groups.iter().all(|g| g.label != "Alice"));
    }
    #[test]
    fn qualified_label_rank_counts_sources_not_occurrences() {
        let mut sources = vec![
            source(3, EntityKind::Technology, "ZetaCommon"),
            source(2, EntityKind::Technology, "ZetaCommon"),
            source(1, EntityKind::Technology, "ZetaCommon"),
        ];
        let mut incidental = source(1, EntityKind::Technology, "AlphaIncidental");
        let mention = incidental.payload.as_ref().unwrap().mentions[0].clone();
        incidental.payload.as_mut().unwrap().mentions = vec![mention; 40];
        sources.push(incidental);
        let members = BTreeSet::from([Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)]);
        let (label, basis) = community_label(&sources, &members, &terms(&sources), true)
            .unwrap_or_else(|e| panic!("{}", e.1));
        assert_eq!(label, "ZetaCommon");
        assert_eq!(basis.source_version_id, Uuid::from_u128(1));
        let narrowed = BTreeSet::from([Uuid::from_u128(1)]);
        assert_eq!(
            community_label(&sources, &narrowed, &terms(&sources), true)
                .unwrap_or_else(|e| panic!("{}", e.1))
                .0,
            "AlphaIncidental"
        );
    }
    #[test]
    fn missing_features_and_isolates_stay_in_every_arm_without_fake_vector_singletons() {
        let mut p = prepared(Mode::EligibleVector);
        p.sources[2].payload = None;
        p.sources[2].candidate = None;
        p.sources[2].request = None;
        p.vectors.available.insert(Uuid::from_u128(3));
        let a = output(&p, &[(101, 1), (102, 1), (103, 2)]).unwrap_or_else(|e| panic!("{}", e.1));
        assert_eq!(a.sources.len(), 3);
        assert_eq!(a.groups.len(), 1);
        assert_eq!(a.groups[0].label_provenance.kind, "canonical_title");
        assert_eq!(a.sources[2].state, "unassigned");
        assert!(a.sources[2].vector_feature_available);
        assert!(!a.sources[2].ner_feature_available);
        p.mode = Mode::EntityOverlap;
        let b = output(&p, &[]).unwrap_or_else(|e| panic!("{}", e.1));
        assert_eq!(b.groups.len(), 1);
        assert_eq!(b.sources[2].state, "unassigned");
    }
    #[test]
    fn metadata_uses_qualified_fields_and_rejects_foreign_or_fake_span_memberships() {
        let mut p = prepared(Mode::Metadata);
        p.metadata_groups.push(Group {
            id: "ignored".into(),
            kind: "ignored".into(),
            label: "PROD".into(),
            source_versions: vec![Uuid::from_u128(2), Uuid::from_u128(1)],
            label_provenance: LabelProvenance {
                kind: "canonical_scope".into(),
                source_version_id: Uuid::from_u128(1),
                byte_start: None,
                byte_end: None,
                quote: None,
                field_kind: Some("environment".into()),
                field_id: Some(Uuid::from_u128(500)),
            },
        });
        let a = output(&p, &[]).unwrap_or_else(|e| panic!("{}", e.1));
        assert_eq!(
            a.groups[0].source_versions,
            vec![Uuid::from_u128(1), Uuid::from_u128(2)]
        );
        assert_eq!(a.sources[2].state, "unassigned");
        p.metadata_groups[0].label_provenance.byte_start = Some(0);
        assert!(output(&p, &[]).is_err());
        p.metadata_groups[0].label_provenance.byte_start = None;
        p.metadata_groups[0]
            .source_versions
            .push(Uuid::from_u128(99));
        assert!(output(&p, &[]).is_err());
    }
    #[test]
    fn foreign_assignments_duplicate_sources_and_payload_versions_fail_closed() {
        let mut p = prepared(Mode::NativeWeighted);
        assert!(output(&p, &[(999, 1)]).is_err());
        p.sources[0].payload.as_mut().unwrap().source_version_id = Uuid::nil();
        assert!(output(&p, &[]).is_err());
        p.sources[0].payload.as_mut().unwrap().source_version_id = p.sources[0].version;
        p.sources.push(p.sources[0].clone());
        assert!(output(&p, &[]).is_err());
    }
}
