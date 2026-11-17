//! Curated, high-fidelity explanation of the first paper in the collection.
//!
//! arXiv:2609.10117 — "Understanding the Security Boundary of Obfuscation-based
//! On-Device LLM Protection" (CCS '26). Content below was extracted from the
//! paper's sections, tables and research questions.

use crate::categories::categorize;
use crate::models::*;
use crate::util::make_id;
use chrono::Utc;

pub const SEED_ID: &str = "2609-10117";
pub const SEED_URL: &str = "https://arxiv.org/html/2609.10117v2";

pub fn seed_paper() -> Paper {
    let now = Utc::now();
    let mut paper = Paper {
        id: SEED_ID.into(),
        title: "Understanding the Security Boundary of Obfuscation-based On-Device LLM Protection".into(),
        authors: vec![
            "Hanyi Zhou".into(), "Chenyang Li".into(), "Yuanzhe Pang".into(),
            "Ke Xu".into(), "Mingwei Xu".into(), "Zhuotao Liu".into(),
        ],
        year: Some(2026),
        url: SEED_URL.into(),
        venue: Some("ACM CCS '26".into()),
        abstract_text: abstract_text(),
        summary: summary(),
        tags: vec![
            "security".into(), "on-device LLM".into(), "TEE".into(), "obfuscation".into(),
            "model extraction".into(), "systematization".into(), "CCS '26".into(),
        ],
        categories: Vec::new(),
        primary_category: String::new(),
        status: PaperStatus::Ready,
        source: PaperSource::Curated,
        added_at: now,
        updated_at: now,
        hypotheses: hypotheses(),
        experiments: experiments(),
        findings: findings(),
        diagrams: diagrams(),
        workflows: workflows(),
        concepts: concepts(),
        sections: sections(),
        notes: vec![
            "Curated from the full paper, including Tables 1–9 and Appendices C–F.".into(),
            "The authors explicitly do not claim universal security for O_ext — see the caveat on the key finding.".into(),
        ],
        enhancement: None,
    };
    let (categories, primary) = categorize(&paper);
    paper.categories = categories;
    paper.primary_category = primary;
    paper
}

fn abstract_text() -> String {
    "On-device LLM deployment exposes valuable weights to a local adversary. TEE-Shielded LLM Partition (TSLP) schemes protect them by keeping non-linear operators in a trusted enclave and offloading weight-bearing linear layers, obfuscated, to an untrusted GPU/NPU. Prior schemes and attacks are largely ad hoc. We systematize them with four linear obfuscation primitives — sparse mask S, low-rank mask L, column permutation Π and column scaling D — and prove that the security boundary of their efficiency-preserving compositions is the canonical form O_prior(W) = (W+S+L)·D·Π. We show this form leaks column directions and build Collapse, a three-stage attack that recovers a surrogate model with near-white-box accuracy using under 1% of task labels. We then add sparse multiplicative and double-sided multiplicative primitives to obtain O_ext, which breaks the invariants Collapse relies on and reduces the surrogate to the empirical black-box reference at comparable runtime cost.".into()
}

fn summary() -> String {
    "The problem: an on-device LLM is a valuable secret, but the device owner controls the OS and GPU. The standard fix is a split: keep small, non-linear work inside a hardware enclave (TEE) and send big matrix multiplications to the GPU (REE) with the weights disguised.

What the authors do: they show that eight seemingly different published defenses are really just combinations of four cheap algebraic tricks — add a few random entries (S), add a low-rank matrix (L), shuffle columns (Π), or rescale columns (D). They prove every efficient combination collapses into one formula, O_prior = (W+S+L)·D·Π, and call that formula the 'security boundary' of prior work.

The bad news: that formula always leaves each weight column's *direction* visible. The paper turns this into Collapse, a three-stage attack that lines up obfuscated views, removes the low-rank masks, and fine-tunes a copy of the public pretrained model using less than 1% of the labels. Across four models it reaches 1.45× the black-box baseline — almost the 1.46× white-box ceiling.

The good news: adding two new primitives that mix columns and rows (O_ext) breaks the invariants Collapse needs. The attack then lands at 1.00× black-box on average, and the extra obfuscation costs about the same as the strongest prior defense (903 ms vs 923 ms per block pair). The paper is careful: this is empirical resistance to the evaluated Collapse pipeline, not a universal security proof.".into()
}

fn hypotheses() -> Vec<Hypothesis> {
    vec![
        Hypothesis {
            id: "hyp:primitives-unify".into(),
            statement: "Existing TSLP obfuscation schemes, though presented as unrelated designs, are all compositions of a small set of efficiently computable linear primitives.".into(),
            simplified: "If you look closely at prior defenses, they are all built from the same four cheap matrix tricks — so they can be compared in one framework instead of one paper at a time.".into(),
            status: HypothesisStatus::Supported,
            evidence_ids: vec!["find:four-primitives".into()],
            experiment_ids: vec!["exp:systematization".into()],
            section: Some("§3 — Systematizing Linear Obfuscation".into()),
            confidence: Some("Proved by algebraic rewriting rules (Rules 2–8)".into()),
        },
        Hypothesis {
            id: "hyp:column-directions-leak".into(),
            statement: "The canonical composition O_prior(W) = (W+S+L)·D·Π cannot hide the column directions of the victim weights, so a high-utility surrogate can be recovered from exposed obfuscated views.".into(),
            simplified: "No matter how you shuffle, scale, sparsely mask or low-rank mask a weight matrix, the 'pointing direction' of each column survives. That is enough to rebuild a model that behaves almost identically to the secret one.".into(),
            status: HypothesisStatus::Supported,
            evidence_ids: vec!["find:collapse-near-whitebox".into(), "find:stages-100".into()],
            experiment_ids: vec!["exp:rq1-attack".into(), "exp:rq2-stages".into()],
            section: Some("§4–§5 — Attack Intuition & Collapse Pipeline".into()),
            confidence: Some("Three structural insights + near-white-box attack results".into()),
        },
        Hypothesis {
            id: "hyp:key-refresh-weakens".into(),
            statement: "Frequently refreshing obfuscation keys can weaken resistance, because comparing multiple differently obfuscated views reveals complementary clues about the weights.".into(),
            simplified: "Counterintuitively, changing the secret more often gives the attacker more puzzle pieces. Each fresh view leaks a different sliver of structure, and the pieces fit together.".into(),
            status: HypothesisStatus::Supported,
            evidence_ids: vec!["find:key-refresh".into()],
            experiment_ids: vec!["exp:rq2-stages".into()],
            section: Some("§8 — Discussion; Appendix F".into()),
            confidence: Some("Analyzed in the fully-refreshed setting (Appendix F)".into()),
        },
        Hypothesis {
            id: "hyp:mixing-extends-boundary".into(),
            statement: "Adding sparse multiplicative and double-sided multiplicative primitives removes the row/column invariants exploited by Collapse, extending the security boundary without a large efficiency penalty.".into(),
            simplified: "If you also mix columns together and mix rows, the attacker can no longer match individual columns. The new defense, O_ext, pushes the attack back to the 'no leaked weights' baseline while costing about the same as before.".into(),
            status: HypothesisStatus::PartiallySupported,
            evidence_ids: vec!["find:oext-blackbox".into(), "find:cost-parity".into(), "find:attack-specific".into()],
            experiment_ids: vec!["exp:rq3-defense".into(), "exp:rq4-cost".into()],
            section: Some("§6–§7 — Extended Boundary & Evaluation".into()),
            confidence: Some("Empirical and attack-specific; not a formal guarantee".into()),
        },
    ]
}

fn experiments() -> Vec<Experiment> {
    vec![
        Experiment {
            id: "exp:systematization".into(),
            name: "Systematization of prior defenses".into(),
            question: "Can representative TSLP defenses be expressed as compositions of a few obfuscation primitives?".into(),
            design: "The authors define a valid primitive by three properties — correctness (recovery returns XW exactly), efficiency (TEE work ≪ matrix multiply), and extraction resistance (a formal game with a 5-point accuracy tolerance). They then rewrite each published scheme's weight transformation using algebraic identities until it reaches a canonical form.".into(),
            setup: ExperimentSetup {
                models: vec!["Transformer linear layers".into()],
                datasets: vec![],
                metrics: vec!["TEE cost".into(), "correctness".into()],
                baselines: vec![],
                conditions: vec!["S: sparse, nnz ≪ n²".into(), "L: rank r ≪ n".into(), "Π: permutation".into(), "D: diagonal".into()],
                budget: None,
            },
            steps: vec![
                step("s1", "Define validity", "Require exact correctness, negligible TEE cost, and resistance in a model-extraction game (δ = 0.05).", "algebra", "definition"),
                step("s2", "Collect schemes", "Take the matrix-level transformations of seven published TSLP defenses.", "literature", "scheme list"),
                step("s3", "Rewrite algebraically", "Apply Rules 2–8: fuse D and Π, push multipliers through additive masks, merge L and S terms.", "scheme list", "canonical forms"),
                step("s4", "State the boundary", "Every efficiency-preserving composition equals O_prior = (W+S+L)·D·Π for some parameters.", "canonical forms", "O_prior"),
            ],
            results: vec![
                "All seven schemes (NNSplitter, Magnitude, LoRO, TransLinkGuard, Soter, TSQP, ArrowCloak) map onto {S, L, Π, D} — Table 2.".into(),
                "O_prior is the most secure scheme attainable from the prior primitive set.".into(),
            ],
            finding_ids: vec!["find:four-primitives".into()],
            hypothesis_ids: vec!["hyp:primitives-unify".into()],
            mermaid: Some("flowchart LR\n  A[\"Define valid primitive<br/>correct · cheap · resistant\"] --> B[\"Collect 7 prior schemes\"]\n  B --> C[\"Apply rewriting rules 2–8\"]\n  C --> D[\"Canonical form<br/>O_prior = (W+S+L)·D·Π\"]\n".into()),
            section: Some("§3".into()),
        },
        Experiment {
            id: "exp:rq1-attack".into(),
            name: "RQ1 · Attack performance across defenses".into(),
            question: "How effective is Collapse against existing defenses?".into(),
            design: "Deploy each defense, refresh L/D/Π at each key epoch while keeping S static, then run Collapse with only 1% of task labels. Compare surrogate accuracy with a white-box upper bound, a black-box lower reference and the prior ArrowMatch attack. Also try the naive shortcut of fine-tuning directly on the obfuscated weights (M_obf).".into(),
            setup: ExperimentSetup {
                models: vec!["BERT-Base (110M)".into(), "ViT-Base (86M)".into(), "Qwen2.5-0.5B (490M)".into(), "Qwen2.5-1.5B (1.54B)".into()],
                datasets: vec!["MNLI".into(), "QNLI".into(), "SST-2".into(), "CIFAR-100".into()],
                metrics: vec!["accuracy".into(), "Rel.Black".into()],
                baselines: vec!["White-box".into(), "Black-box".into(), "ArrowMatch".into(), "M_obf fine-tuning".into()],
                conditions: vec!["5 prior defenses".into(), "O_prior".into(), "static S, dynamic L/D/Π".into()],
                budget: Some("< 1% of original training data".into()),
            },
            steps: vec![
                step("s1", "Deploy defenses", "Instantiate NNSplitter, LoRO, TSQP, TransLinkGuard, ArrowCloak and O_prior on each model.", "model + defense", "obfuscated views"),
                step("s2", "Collect views", "Record every tensor crossing the TEE–REE boundary across key refreshes.", "runtime", "T views"),
                step("s3", "Run Collapse", "Align permutations, remove low-rank masks, align to public prior, fine-tune on 1% task data.", "T views + W_pre", "surrogate M_sur"),
                step("s4", "Measure", "Report accuracy and Rel.Black = Acc(M_sur) / Acc(black-box).", "M_sur", "RQ1 numbers"),
            ],
            results: vec![
                "Collapse averages 90.54% accuracy = 1.45× black-box, versus the white-box ceiling of 91.11% = 1.46× — Table 5.".into(),
                "Fine-tuning directly on obfuscated weights averages ~40% (0.63–0.66×): worse than black-box fine-tuning.".into(),
                "Holds across all four models and all six defense configurations, including full O_prior.".into(),
            ],
            finding_ids: vec!["find:collapse-near-whitebox".into(), "find:obf-worse-than-blackbox".into()],
            hypothesis_ids: vec!["hyp:column-directions-leak".into()],
            mermaid: None,
            section: Some("§7.2 — Table 5".into()),
        },
        Experiment {
            id: "exp:rq2-stages".into(),
            name: "RQ2 · Stage-wise attack success".into(),
            question: "How accurately does Collapse recover the target obfuscation component at each stage?".into(),
            design: "Run Collapse three times on BERT-Base/MNLI and compare each stage's output with ground truth: relative permutation alignment (Stage 1), similarity of the recovered low-rank mask (Stage 2), and absolute permutation plus sparse-support detection (Stage 3).".into(),
            setup: ExperimentSetup {
                models: vec!["BERT-Base (m = n = 768, r = 2)".into()],
                datasets: vec!["MNLI".into()],
                metrics: vec!["relative Π accuracy".into(), "L-similarity".into(), "absolute Π accuracy".into(), "support P/R/F1".into()],
                baselines: vec!["ground-truth obfuscation components".into()],
                conditions: vec!["3 independent runs, mean reported".into(), "LDD oracle with k = 32".into()],
                budget: None,
            },
            steps: vec![
                step("s1", "Stage 1 — align", "Use the LDD oracle (rank of stacked column subsets) to seed, disambiguate and propagate a column correspondence across ≥ 4 views.", "T views", "common column order"),
                step("s2", "Stage 2 — unmask", "Intersect mask subspaces from view pairs (0,1) and (2,3), project away L, solve for relative scales.", "aligned views", "W+S estimate"),
                step("s3", "Stage 3 — reconstruct", "Match columns to W_pre by cosine similarity, find the sparse support, rescale, fine-tune.", "W+S + W_pre + 1% data", "M_sur"),
                step("s4", "Verify each stage", "Compare recovered components against the true Π, L, S and column ordering.", "stage outputs", "recovery metrics"),
            ],
            results: vec![
                "Stage 1 relative permutation accuracy: 100.00%.".into(),
                "Stage 2 L-Sim: 99.97%.".into(),
                "Stage 3 absolute permutation accuracy: 100.00%; sparse support P/R/F1 = 99.36% / 99.22% / 99.29% — Table 6.".into(),
            ],
            finding_ids: vec!["find:stages-100".into(), "find:key-refresh".into()],
            hypothesis_ids: vec!["hyp:column-directions-leak".into(), "hyp:key-refresh-weakens".into()],
            mermaid: Some(CURATED_DIAGRAMS[2].to_string()),
            section: Some("§7.3 — Table 6".into()),
        },
        Experiment {
            id: "exp:rq3-defense".into(),
            name: "RQ3 · Defense effectiveness of O_ext".into(),
            question: "How effective is O_ext as a defense against Collapse?".into(),
            design: "Run the exact same Collapse pipeline and hyperparameters from RQ1 against O_ext, with no defense-specific adaptation. Compare with the best prior defense (ArrowCloak). Black-box here is an empirical reference obtained without using obfuscated weights, not a formal security bound.".into(),
            setup: ExperimentSetup {
                models: vec!["ViT-Base".into(), "BERT-Base".into(), "Qwen2.5-0.5B".into(), "Qwen2.5-1.5B".into()],
                datasets: vec!["CIFAR-100".into(), "MNLI".into(), "QNLI".into(), "SST-2".into()],
                metrics: vec!["surrogate accuracy".into(), "Rel.Black".into()],
                baselines: vec!["Prior Best (ArrowCloak)".into(), "Black-box reference".into()],
                conditions: vec!["same Collapse config as RQ1".into(), "no adaptive attack".into()],
                budget: Some("1% task data".into()),
            },
            steps: vec![
                step("s1", "Add mixing primitives", "Obfuscate with O_ext(X,W) = X·M_X and M_X⁻¹·(W+L+S)·M_W.", "W, L, S, M_X, M_W", "O_ext views"),
                step("s2", "Run unchanged Collapse", "Apply the RQ1 attack verbatim — its row/column invariants no longer hold.", "O_ext views", "surrogate"),
                step("s3", "Compare", "Report accuracy and Rel.Black against Prior Best and the black-box reference.", "surrogate", "RQ3 numbers"),
            ],
            results: vec![
                "O_ext averages 62.23% = 1.00× black-box, versus Prior Best 89.67% = 1.46× — Table 7.".into(),
                "Strongest effect on ViT-Base/CIFAR-100: 53.40% = 0.78× (Prior Best 89.44% = 1.31×).".into(),
                "This is a 27.4 percentage-point average reduction in surrogate accuracy.".into(),
            ],
            finding_ids: vec!["find:oext-blackbox".into(), "find:attack-specific".into()],
            hypothesis_ids: vec!["hyp:mixing-extends-boundary".into()],
            mermaid: None,
            section: Some("§7.4 — Table 7".into()),
        },
        Experiment {
            id: "exp:rq4-cost".into(),
            name: "RQ4 · Cost of O_ext".into(),
            question: "What cost does O_ext impose relative to prior defenses?".into(),
            design: "Prototype a TEE–GPU system: Python inside Occlum for the enclave, a long-lived GPU worker, socket communication. Microbenchmark a 768×768 BERT-Base linear layer at batch 1, sequence length 768 (10-sample mean), split into obfuscation, GPU compute, TEE↔GPU transfer and recovery. Aggregate over one self-attention and one feed-forward block.".into(),
            setup: ExperimentSetup {
                models: vec!["BERT-Base 768×768 linear layer".into()],
                datasets: vec![],
                metrics: vec!["end-to-end latency (ms)".into(), "primitive cost (ms)".into()],
                baselines: vec!["ArrowCloak".into(), "O_prior".into()],
                conditions: vec!["batch = 1".into(), "sequence length = 768".into(), "10-sample mean".into(), "TEE: Python on Occlum".into()],
                budget: None,
            },
            steps: vec![
                step("s1", "Build prototype", "Run Python in Occlum (TEE), a GPU worker process, and socket transport between them.", "TEE + GPU host", "testbed"),
                step("s2", "Microbenchmark primitives", "Time sparse mask, low-rank mask (r=1/2/4), scaling, permutation and sparse mixing separately.", "testbed", "Table 9"),
                step("s3", "Aggregate blocks", "Sum obfuscation, GPU compute, transfer and recovery for one attention + one FFN block.", "primitives", "Table 8"),
            ],
            results: vec![
                "End-to-end: O_ext 903.33 ms vs O_prior 855.82 ms vs ArrowCloak 922.81 ms — Table 8.".into(),
                "Sparse mixing: 10.97 ms obfuscation / 0.23 ms GPU / 11.61 ms recovery — Table 9.".into(),
                "New primitives stay in the same order of magnitude as permutation + scaling combined.".into(),
            ],
            finding_ids: vec!["find:cost-parity".into()],
            hypothesis_ids: vec!["hyp:mixing-extends-boundary".into()],
            mermaid: None,
            section: Some("§7.5 — Tables 8–9".into()),
        },
    ]
}

fn step(id: &str, title: &str, detail: &str, input: &str, output: &str) -> WorkflowStep {
    WorkflowStep {
        id: id.into(), title: title.into(), detail: detail.into(),
        inputs: vec![input.into()], outputs: vec![output.into()],
    }
}

fn findings() -> Vec<Finding> {
    vec![
        Finding {
            id: "find:four-primitives".into(),
            claim: "The matrix-level transformations of seven representative TSLP schemes all belong to the family generated by four primitives {S, L, Π, D}, whose security boundary is O_prior = (W+S+L)·D·Π.".into(),
            simplified: "Seven different 'novel' defenses are actually just different recipes made from the same four ingredients. The strongest recipe you can cook with those ingredients has one formula.".into(),
            evidence: "Table 1 (primitives), Table 2 (scheme mapping), Rules 2–8".into(),
            experiment_id: Some("exp:systematization".into()), importance: Importance::High, caveat: None,
        },
        Finding {
            id: "find:collapse-near-whitebox".into(),
            claim: "Collapse reaches 1.45× the black-box baseline on average, essentially matching the 1.46× white-box upper bound across four models and four datasets.".into(),
            simplified: "The attack gets almost all the way to 'attacker has the real model' performance, even though its obfuscated weights alone are useless for training.".into(),
            evidence: "Table 5: Collapse 90.54% avg / 1.45×; White-box 91.11% / 1.46×; Black-box 62.51%".into(),
            experiment_id: Some("exp:rq1-attack".into()), importance: Importance::High, caveat: None,
        },
        Finding {
            id: "find:obf-worse-than-blackbox".into(),
            claim: "Fine-tuning directly on obfuscated weights performs worse than fine-tuning from the public pretrained model alone (about 0.63–0.66× vs 1.0× black-box).".into(),
            simplified: "Handing the attacker the scrambled weights is not helpful to them — it is actively a bad starting point. The paper's attack wins by *repairing* those weights, not by using them raw.".into(),
            evidence: "Table 5: M_obf averages ≈ 39.7–41.1% (0.63–0.66×) vs black-box 62.51%".into(),
            experiment_id: Some("exp:rq1-attack".into()), importance: Importance::High, caveat: None,
        },
        Finding {
            id: "find:stages-100".into(),
            claim: "Every stage of Collapse recovers its target component at approximately 100% fidelity, so the attack does not rely on downstream fine-tuning alone.".into(),
            simplified: "Each step of the three-step attack does its job almost perfectly — the final fine-tuning is cleanup, not theheavy lifting.".into(),
            evidence: "Table 6: Stage 1 Π 100.00%; Stage 2 L-Sim 99.97%; Stage 3 Π 100.00%, support F1 99.29%".into(),
            experiment_id: Some("exp:rq2-stages".into()), importance: Importance::High, caveat: Some("Measured on BERT-Base/MNLI with r = 2; 3-run mean.".into()),
        },
        Finding {
            id: "find:oext-blackbox".into(),
            claim: "Under the evaluated Collapse configuration, O_ext reduces average surrogate accuracy to 62.23% (1.00× black-box) versus 89.67% (1.46×) for the strongest prior defense — a 27.4 percentage-point drop.".into(),
            simplified: "With the new mixing tricks, the attacker ends up with no more information than someone who never saw the protected weights at all (on average).".into(),
            evidence: "Table 7: O_ext avg 62.23% / 1.00×; Prior Best avg 89.67% / 1.46×; ViT-Base 53.40% / 0.78×".into(),
            experiment_id: Some("exp:rq3-defense".into()), importance: Importance::High,
            caveat: Some("Empirical and attack-specific: no defense-adapted attack was tried. Black-box is an observed reference, not a formal lower bound.".into()),
        },
        Finding {
            id: "find:key-refresh".into(),
            claim: "More frequent obfuscation key rotation can weaken resistance under this threat model, because comparing multiple obfuscated views reveals complementary structural clues.".into(),
            simplified: "Refreshing the secret more often is counterproductive here: every new view is another clue for the attacker to cross-check against the others.".into(),
            evidence: "§8 'Key rotation and boundary-visible leakage'; Appendix F (fully refreshed setting)".into(),
            experiment_id: Some("exp:rq2-stages".into()), importance: Importance::Medium,
            caveat: Some("Analysis is specific to the algebraic leakage channel studied here.".into()),
        },
        Finding {
            id: "find:cost-parity".into(),
            claim: "O_ext's end-to-end overhead is on par with prior defenses: 903.33 ms versus 855.82 ms for O_prior and 922.81 ms for ArrowCloak per attention+FFN block pair.".into(),
            simplified: "The stronger defense is not slower in practice: its extra work is a few cheap sparse passes that fit inside the existing cost envelope.".into(),
            evidence: "Table 8 (block totals); Table 9: sparse mixing 10.97 ms obf / 11.61 ms recovery".into(),
            experiment_id: Some("exp:rq4-cost".into()), importance: Importance::Medium,
            caveat: Some("Prototype measured on a 768×768 BERT layer; GPU/TEE hardware details in §7.5.".into()),
        },
        Finding {
            id: "find:attack-specific".into(),
            claim: "The authors do not claim universal security for O_ext: it is evaluated only against the existing Collapse pipeline, without a defense-specific adaptive attack.".into(),
            simplified: "Important honesty note: the new defense beat the attack they tried, but nobody has tried a attack designed specifically against the new defense yet. So treat 'secure' as 'resisted this particular attack'.".into(),
            evidence: "§1 contributions caveat; §8 'Defense scope and adaptive attacks'; RQ3 answer".into(),
            experiment_id: Some("exp:rq3-defense".into()), importance: Importance::High,
            caveat: Some("Developing defense-aware attacks against M_X and M_W is left to future work.".into()),
        },
    ]
}

fn diagrams() -> Vec<Diagram> {
    let d = |id: &str, title: &str, caption: &str, mermaid: &str, kind: DiagramKind| Diagram {
        id: id.into(), title: title.into(), caption: caption.into(), mermaid: mermaid.into(), kind, paper_id: SEED_ID.into(),
    };
    vec![
        d("dia:tee-ree", "TEE–REE collaborative inference", "Where the secret weights live, what the untrusted GPU sees, and how the enclave restores the correct result.", CURATED_DIAGRAMS[0], DiagramKind::Architecture),
        d("dia:primitives", "The four obfuscation primitives", "Additive masks (S, L) and multiplicative maps (Π, D) compose into the canonical boundary O_prior.", CURATED_DIAGRAMS[1], DiagramKind::Taxonomy),
        d("dia:collapse", "The Collapse attack pipeline", "Recreated from §5: three stages that turn leaked column directions into a surrogate model.", CURATED_DIAGRAMS[2], DiagramKind::Workflow),
        d("dia:leakage", "Why O_prior leaks", "The three structural channels that keep weight directions visible despite obfuscation.", CURATED_DIAGRAMS[3], DiagramKind::Leakage),
        d("dia:boundary", "Extending the security boundary", "From the prior primitive set to O_ext, and the measured effect on attack success.", CURATED_DIAGRAMS[4], DiagramKind::Timeline),
        d("dia:evaluation", "Evaluation workflow", "Models → datasets → defenses → baselines → attack → RQs, as recreated from §7.", CURATED_DIAGRAMS[5], DiagramKind::Workflow),
    ]
}

fn workflows() -> Vec<Workflow> {
    vec![
        Workflow {
            id: "wf:tee-ree".into(),
            title: "Obfuscation & recovery at the TEE–REE boundary".into(),
            description: "How an enclave protects a linear layer while still using an untrusted accelerator. This is the system every defense in the paper fits into.".into(),
            category: "defense".into(),
            paper_id: SEED_ID.into(),
            mermaid: CURATED_DIAGRAMS[0].into(),
            steps: vec![
                step("s1", "Keep the secret in the TEE", "The proprietary weight W and the non-linear operators (LayerNorm, Softmax, residual) stay inside the hardware enclave.", "model weights W", "protected weights"),
                step("s2", "Obfuscate before offloading", "Sample S, L, D, Π and form W̃ = (W+S+L)·D·Π. Only this disguised matrix leaves the enclave.", "W + keys", "obfuscated W̃"),
                step("s3", "Compute in the REE", "The GPU/NPU multiplies the input by W̃. It sees inputs, W̃ and the product — but not W or the corrected output.", "X, W̃", "X·W̃"),
                step("s4", "Recover inside the TEE", "Undo the transforms in reverse: R(X·W̃) = X·W̃·Π⁻¹·D⁻¹ − X·L − X·S = X·W.", "X·W̃", "correct X·W"),
            ],
        },
        Workflow {
            id: "wf:collapse".into(),
            title: "Collapse: three-stage model extraction".into(),
            description: "The recreated attack from §5. Stages 1 and 2 run per weight matrix; Stage 3 runs once on the assembled model.".into(),
            category: "attack".into(),
            paper_id: SEED_ID.into(),
            mermaid: CURATED_DIAGRAMS[2].into(),
            steps: vec![
                step("s1", "Collect T obfuscated views", "Record W̃_t = (W+S+L_t)·D_t·Π_t across key refreshes; S is static, L/D/Π change.", "device boundary", "T views"),
                step("s2", "Stage 1 · align columns", "Use the LDD oracle (rank of stacked column subsets) to seed, disambiguate and propagate a bijection between views. Cost: O(n²) oracle calls.", "T views", "K ≥ 4 aligned views"),
                step("s3", "Stage 2 · remove low-rank masks", "Intersect mask subspaces of view pairs, project away L, and solve for relative scales to obtain an anchor-ordered W+S estimate.", "aligned views", "W+S estimate"),
                step("s4", "Stage 3 · rebuild the model", "Match columns to the public W_pre by cosine similarity, estimate the sparse support, rescale, and fine-tune on < 1% task data.", "W+S, W_pre, 1% data", "surrogate M_sur"),
            ],
        },
        Workflow {
            id: "wf:evaluation".into(),
            title: "How the evaluation was run".into(),
            description: "The experimental recipe from §7, recreated so results can be reproduced or adapted to a new defense.".into(),
            category: "evaluation".into(),
            paper_id: SEED_ID.into(),
            mermaid: CURATED_DIAGRAMS[5].into(),
            steps: vec![
                step("s1", "Choose models and tasks", "Four models from 86M to 1.54B parameters, spanning auto-encoding, vision and decoder-only architectures; four datasets.", "checkpoints", "evaluation matrix"),
                step("s2", "Deploy defenses and baselines", "Five prior schemes plus O_prior; compare with white-box, black-box, M_obf and ArrowMatch.", "defenses", "deployed targets"),
                step("s3", "Run attacks and measurements", "Execute Collapse (RQ1–RQ3) and a TEE–GPU prototype microbenchmark (RQ4).", "targets", "raw measurements"),
                step("s4", "Report accuracy and Rel.Black", "Normalize surrogate accuracy by the black-box reference and answer RQ1–RQ4.", "measurements", "Tables 5–9"),
            ],
        },
    ]
}

fn concepts() -> Vec<Concept> {
    let c = |name: &str, definition: &str, category: &str| Concept {
        id: make_id("concept", name), name: name.into(), definition: definition.into(),
        category: category.into(), aliases: vec![],
    };
    vec![
        c("TEE", "Trusted Execution Environment: hardware-isolated execution (e.g. SGX, TrustZone) that protects code and data even when the OS is untrusted. Secure memory and throughput are limited.", "primitive"),
        c("REE", "Rich Execution Environment: the untrusted side of the device, here the commodity GPU/NPU that runs the offloaded matrix multiplications.", "primitive"),
        c("TSLP", "TEE-Shielded LLM Partition: the deployment pattern where lightweight non-linear operators stay in the TEE and weight-bearing linear layers are obfuscated and offloaded.", "concept"),
        c("Obfuscation primitive", "A pair of operators (O, R) that disguises a weight matrix for offloading and restores the exact product XW in the enclave. Valid iff correct, efficient, and extraction-resistant.", "primitive"),
        c("Sparse mask", "Primitive P_S: add a mask S with very few nonzeros (nnz ≪ n²). Recovery subtracts X·S. Used by NNSplitter and Magnitude.", "primitive"),
        c("Low-rank mask", "Primitive P_L: add L = A·Bᵀ with rank r ≪ n. Recovery subtracts (X·A)·Bᵀ. Used by LoRO and ArrowCloak.", "primitive"),
        c("Column permutation", "Primitive P_Π: right-multiply by a permutation matrix Π. Recovery permutes columns back — essentially a memory copy.", "primitive"),
        c("Column scaling", "Primitive P_D: right-multiply by a non-singular diagonal D. Recovery divides each column by its scale; costs O(mn).", "primitive"),
        c("O_prior", "The security boundary of prior primitives: O_prior(W) = (W+S+L)·D·Π. Every efficiency-preserving composition of {S, L, Π, D} is an instance of it.", "concept"),
        c("Collapse", "The paper's primitive-guided model-extraction attack: align views, remove low-rank masks, then fine-tune a surrogate from the public prior with < 1% task labels.", "attack"),
        c("LDD oracle", "Linear Dependency Detection oracle: a rank test on stacked column subsets that tells whether two views' selected columns correspond to the same victim columns.", "attack"),
        c("Sparse multiplicative primitive", "P_M: multiply W by a mixing matrix M = Σ P_i·D_i with a sparse inverse. Breaks column atomicity by making each exposed column a mix of several victim columns.", "primitive"),
        c("Double-sided primitive", "P_DS: also transform the input with M_X and the weight with M_X⁻¹·W·M_W, hiding row directions as well as column directions.", "primitive"),
        c("O_ext", "The extended boundary: O_X(X)=X·M_X and O_W(W)=M_X⁻¹·(W+L+S)·M_W. Subsumes O_prior and removes the invariants Collapse exploits.", "concept"),
        c("Surrogate model", "A model M_sur with the victim's architecture and comparable downstream accuracy, built without exactly recovering every parameter.", "concept"),
        c("Model extraction", "The security game: with T observations and a labeled budget B, an attacker wins if surrogate accuracy is within δ = 0.05 of the victim's.", "concept"),
    ]
}

fn sections() -> Vec<PaperSection> {
    let s = |heading: &str, level: u8, paras: &[&str]| PaperSection {
        heading: heading.into(), level, paragraphs: paras.iter().map(|p| p.to_string()).collect(),
    };
    vec![
        s("1. Introduction", 2, &[
            "On-device LLMs reduce latency and keep data local, but a local adversary has white-box visibility into code, parameters, memory and activations. TEE-Shielded LLM Partition (TSLP) keeps non-linear operators in an enclave and offloads linear layers with obfuscated weights.",
            "Prior schemes and attacks are ad hoc: each invents algebraic tricks and shows a handful of obvious inversions fail. This paper asks whether common primitives can unify them, characterize the security boundary of their compositions, and systematically extend it.",
            "Contributions: unified obfuscation primitives; identify the boundary O_prior; break it with Collapse; extend it with O_ext.",
        ]),
        s("2. Background and Threat Model", 2, &[
            "A transformer block has three operator classes: linear (weight-bearing QKV/O/FFN), quadratic (token-to-token attention products) and non-poly (normalization, activation, residual). The TEE keeps non-poly work and offloads linear + quadratic work.",
            "Threat model: honest-but-curious. The protocol runs correctly, but the device owner records every exposed view W̃_t across key refreshes, can query for < 1% labeled task data, and knows the public pretrained backbone identified by fingerprinting. Hardware side channels are out of scope.",
            "The attacker's goal is high-utility surrogate M_sur, not exact recovery of every parameter.",
        ]),
        s("3. Systematizing Linear Obfuscation", 2, &[
            "A valid primitive must satisfy correctness R(X·O(W)) = XW, efficiency C(O)+C(R) ≪ C(XW), and model-extraction resistance under a game with δ = 0.05.",
            "Two additive primitives (sparse S, low-rank L) and two multiplicative primitives (permutation Π, scaling D) generate an efficiency-preserving family F(B_prior).",
            "Rewriting rules fuse D and Π, push multipliers through additive masks, and merge same-type masks. The canonical boundary is O_prior(W) = (W+S+L)·D·Π.",
        ]),
        s("4. Attack Intuition", 2, &[
            "Insight 1 (column atomicity): Π and D only relabel and rescale columns; each victim column survives as a transformed copy of one original column.",
            "Insight 2 (sparse coverage gap): S has nnz ≪ n², so nearly every entry is released verbatim; corrupting a few entries barely tilts a column direction.",
            "Insight 3 (low-rank coverage gap): L lives in an r-dimensional subspace; projecting it away exposes the other n−r dimensions noise-free.",
        ]),
        s("5. The Collapse Pipeline", 2, &[
            "Attacker observes T views W̃_t = (W+S+L_t)·D_t·Π_t with static S and dynamic L, D, Π. Stage 1 aligns relative permutations using the LDD oracle; Stage 2 intersects mask subspaces to remove L; Stage 3 aligns to W_pre and fine-tunes.",
            "The LDD oracle stacks k columns from two views and uses the rank of the result to detect overlap: full overlap gives rank k+2r, partial overlap gives k+s+2r. With k = 32 and r = 2, overlaps of 4–32 columns are resolved exactly.",
            "Stage 3 matches columns by cosine similarity, estimates the sparse support via normalized residuals, rescales by length ratios, and fine-tunes on the task budget.",
        ]),
        s("6. Extending the Security Boundary", 2, &[
            "The leakage traces to one invariant: column directions survive. P_M mixes columns with M = Σ P_i·D_i; P_DS also mixes rows using M_X and M_W.",
            "The extended family's boundary is O_ext: O_X(X)=X·M_X, O_W(W)=M_X⁻¹·(W+L+S)·M_W. Setting M_X=I and M_W=D·Π recovers O_prior, so O_ext strictly subsumes it.",
            "Per-query overhead stays O(mn); per-refresh materialization stays O(n²), on par with O_prior.",
        ]),
        s("7. Evaluation", 2, &[
            "Models: BERT-Base (110M), ViT-Base (86M), Qwen2.5-0.5B (490M), Qwen2.5-1.5B (1.54B). Datasets: MNLI, SST-2, QNLI, CIFAR-100. Defenses: NNSplitter, LoRO, TSQP, TransLinkGuard, ArrowCloak, O_prior.",
            "RQ1: Collapse averages 90.54% (1.45× black-box) vs white-box 91.11% (1.46×). Using obfuscated weights raw averages ~40% and is worse than black-box.",
            "RQ2: Stage 1 Π 100%, Stage 2 L-Sim 99.97%, Stage 3 Π 100% and support F1 99.29% (BERT-Base/MNLI, 3 runs).",
            "RQ3: O_ext averages 62.23% (1.00× black-box) vs Prior Best 89.67% (1.46×) — a 27.4-point reduction. RQ4: 903.33 ms vs 855.82 ms (O_prior) and 922.81 ms (ArrowCloak).",
        ]),
        s("8. Discussion", 2, &[
            "GPU TEEs are complementary but need specific hardware; side-channel attacks are orthogonal. Key rotation can help the attacker. Evidence for O_ext is empirical and attack-specific; defense-aware attacks are future work.",
        ]),
        s("10. Conclusion", 2, &[
            "A primitive-level abstraction systematizes TSLP defenses, reveals a shared vulnerability via Collapse, and guides an extension that raises empirical resistance at comparable cost.",
        ]),
    ]
}

/// Mermaid sources kept in one place so the seed diagrams and the experiment
/// cards can share them.
pub const CURATED_DIAGRAMS: [&str; 6] = [
    // 0 — TEE/REE architecture
    "flowchart LR\n  X[\"Input tokens X\"] --> S1\n  subgraph TEE[\"TEE - trusted, limited memory\"]\n    W[\"Secret weights W\"] --> O[\"Obfuscate<br/>W~ = (W+S+L)·D·Π\"]\n    R[\"Recover<br/>X·W~ → X·W\"] --> N[\"LayerNorm / Softmax / residual\"]\n  end\n  subgraph REE[\"REE - untrusted GPU / NPU\"]\n    V[\"Exposed view W~\"] --> M[\"Matrix multiply X·W~\"]\n  end\n  O --> V\n  M --> R\n  X --> M\n  classDef secure fill:#eef2ff,stroke:#6366f1,color:#312e81;\n  classDef danger fill:#fff7ed,stroke:#f97316,color:#7c2d12;\n  class W,O,R,N secure;\n  class V,M danger;\n",
    // 1 — primitive taxonomy
    "flowchart TB\n  P[\"Obfuscation primitives B_prior\"] --> A[\"Additive: add structured noise\"]\n  P --> M[\"Multiplicative: invertible linear map\"]\n  A --> S[\"P_S sparse mask<br/>W + S · nnz ≪ n²\"]\n  A --> L[\"P_L low-rank mask<br/>W + A·Bᵀ · rank r ≪ n\"]\n  M --> PI[\"P_Π permutation<br/>W·Π · memcpy\"]\n  M --> D[\"P_D scaling<br/>W·D · O(mn)\"]\n  S --> C[\"Canonical boundary<br/>O_prior = (W+S+L)·D·Π\"]\n  L --> C\n  PI --> C\n  D --> C\n  classDef out fill:#ecfdf5,stroke:#10b981,color:#065f46;\n  class C out;\n",
    // 2 — Collapse pipeline
    "flowchart TD\n  IN[\"T views: W~t = (W+S+Lt)·Dt·Πt\"] --> S1\n  subgraph S1[\"Stage 1 - Cross-view column alignment\"]\n    A1[\"LDD oracle: rank of stacked column subsets\"] --> A2[\"Seed → disambiguate → propagate\"]\n    A2 --> A3[\"K ≥ 4 views in a common column order\"]\n  end\n  S1 --> S2\n  subgraph S2[\"Stage 2 - Mask recovery\"]\n    B1[\"Intersect mask subspaces of view pairs\"] --> B2[\"Project away low-rank L\"]\n    B2 --> B3[\"Anchor-ordered W+S estimate\"]\n  end\n  S2 --> S3\n  subgraph S3[\"Stage 3 - Fine-tuning\"]\n    C1[\"Cosine-match columns to public W_pre\"] --> C2[\"Estimate sparse support\"]\n    C2 --> C3[\"Rescale and fine-tune on < 1% labels\"]\n  end\n  S3 --> OUT[\"Surrogate M_sur ≈ victim M_vic\"]\n  classDef result fill:#ecfdf5,stroke:#10b981,color:#065f46;\n  class OUT result;\n",
    // 3 — leakage channels
    "flowchart TD\n  W[\"Secret weight W\"] --> L1[\"Π and D: columns only relabeled / rescaled<br/>column directions survive\"]\n  W --> L2[\"S: nnz ≪ n²<br/>most entries leak in cleartext\"]\n  W --> L3[\"L: rank r ≪ n<br/>n−r dimensions leak noise-free\"]\n  L1 --> X[\"Collapse exploits the residual column directions\"]\n  L2 --> X\n  L3 --> X\n  X --> Y[\"High-fidelity surrogate model\"]\n  classDef warn fill:#fff7ed,stroke:#f97316,color:#7c2d12;\n  class L1,L2,L3 warn;\n",
    // 4 — boundary evolution
    "flowchart LR\n  A[\"B_prior = {S, L, Π, D}\"] -->|\"Collapse breaks it\"| B[\"O_prior<br/>(W+S+L)·D·Π\"]\n  B -->|\"add P_M sparse mixing\"| C[\"B_ext\"]\n  C -->|\"add P_DS double-sided\"| D[\"O_ext<br/>M_X⁻¹·(W+L+S)·M_W\"]\n  D --> E[\"Row + column directions hidden\"]\n  E --> F[\"Rel.Black 1.46× → 1.00×\"]\n  classDef win fill:#ecfdf5,stroke:#10b981,color:#065f46;\n  class D,E,F win;\n",
    // 5 — evaluation workflow
    "flowchart LR\n  M[\"4 models<br/>86M → 1.54B params\"] --> D[\"4 datasets<br/>MNLI · QNLI · SST-2 · CIFAR-100\"]\n  D --> DF[\"6 configurations<br/>5 defenses + O_prior\"]\n  DF --> B[\"Baselines<br/>white · black · ArrowMatch · M_obf\"]\n  B --> A[\"Run Collapse (RQ1–RQ3)<br/>TEE-GPU prototype (RQ4)\"]\n  A --> R[\"Metrics: accuracy + Rel.Black\"]\n  R --> Q[\"Answer RQ1–RQ4\"]\n",
];
