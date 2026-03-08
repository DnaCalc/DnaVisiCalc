#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecClause {
    pub id: &'static str,
    pub summary: &'static str,
}

pub const FEC_F3E_INTERFACE_VERSION: &str = "fec-f3e-redesign/b4";

pub const FEC_F3E_CLAUSES: &[SpecClause] = &[
    SpecClause {
        id: "FEC-F3E-TXN-001",
        summary: "F3E exposes prepare/execute transactional semantic calls.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-002",
        summary: "FEC coordinates open-session/capability/commit atomically.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-003",
        summary: "Commit status/reject-codes are explicit and machine-classifiable.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-004",
        summary: "Stable identity contract uses formula/name/range IDs; names are metadata only.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-005",
        summary: "CommitResult separates value/shape/topology deltas.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-006",
        summary: "Spill handling is represented as explicit SpillTakeover/SpillClearance/SpillBlocked events.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-007",
        summary: "Commit enforces coordinator snapshot fence in addition to session snapshot equality.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-008",
        summary: "Capability decisions are session-bound and commit-validated against bound authority.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-009",
        summary: "Trace payload is schema-versioned and validates required field integrity.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-010",
        summary: "Seam emits perf scaffolding counters for commit/reject/delta/spill telemetry.",
    },
    SpecClause {
        id: "FEC-F3E-TXN-011",
        summary: "Incremental name invalidation uses runtime name-id dependency routing; FEC does not force full-recalc policy.",
    },
];

pub fn clause_by_id(id: &str) -> Option<SpecClause> {
    FEC_F3E_CLAUSES
        .iter()
        .copied()
        .find(|clause| clause.id == id)
}
