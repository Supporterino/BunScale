# Graph Report - BunScale  (2026-10-22)

## Corpus Check
- 412 files · ~655,461 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 79 file(s) not represented in the graph (top: .csv 53, .css 7, (none) 5)

## Summary
- 4500 nodes · 9713 edges · 225 communities (172 shown, 53 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 425 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `006d8406`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- entry.ts
- scripts/core.py
- gray
- design_system.py
- P1 IPC contract (proposal, architect-owned)
- slide_search_core.py
- Export & Reminders Decisions
- DM-4 An archived rabbit is read-only everywhere
- Health & Photos Design
- commands/identity.rs
- Design Skill References
- local.rs
- html-token-validator.py
- Reimplementation Domain Model
- Component Token Values
- status.rs
- Icon Generation Pipeline
- UI Styling Concepts
- multiple_households.rs
- OpenSpec Workflow Skills
- read_path.rs
- queries.ts
- storage_boundary.rs
- Open items
- failure_surface.rs
- Tailwind Config Tests
- Sharing Runtime Behavior
- lifecycle.rs
- color
- Sharing Delivery Ordering
- Household Invitation Design
- Design Token Layers
- guards.rs
- fontSize
- Slides Copywriting & Template
- Brand Color Extraction
- identity_and_unit.rs
- biome.json
- Catalog Refresh Tests
- OpenSpec CLI Commands
- Brand Asset Validation
- conflict_convergence.rs
- TailwindConfigGenerator
- desktop-settings.spec.ts
- Brand Guidelines References
- tauri.conf.json
- Tailwind Breakpoints Tests
- compilerOptions
- timestamps.rs
- DesignSystemGenerator
- json
- read.rs
- Embed Tokens Script
- Token Validator Script
- card
- css-invariants.ts
- Brand Context Injection
- Motion Token Values
- Tailwind Fonts & Spacing Tests
- package.json
- Brand Token Sync Script
- Generate Tokens Script
- @tanstack/react-router
- scaffold-files.ts
- Component Specs & States
- Rust & Tauri Conventions
- design-tokens-starter.json
- SETUP.md — local development setup
- Weight Drop Alert Rules
- probe.rs
- Requirements
- validate_data.py
- store.rs
- default.json
- ADR 0002 — Shared service mechanism: where the shared service runs and how it is written to and read from
- Release & Definition of Done
- Tauri Security Rules
- radius
- merge.rs
- records.rs
- Frontend IPC & Errors
- Mobile Router & History
- AppState
- ADDED Requirements
- gate.rs
- Weight Validation Rules
- Frontend State & Routing
- Design Dials & Generation
- argparse
- save-failure.spec.ts
- create.rs
- xl
- manifest-pins.ts
- Owner Attribution Sharing
- DASH-3 Rabbits are sorted by most recent weigh-in with never-weighed last
- region.rs
- Slide System & Tokens
- UI Styling Dependencies
- app
- none
- AppState
- household_creation.rs
- service.rs
- Quick Log Loop Rules
- Banner Design Skill
- Graphify Usage Workflow
- Member List Resolution
- Archive order P0 to P1 to P2 to P3 to P5 to P4 to P6
- Dashboard Empty States
- failure.rs
- Card Component Spec
- Dialog Component Spec
- Route Guards
- Dashboard Never-Weighed State
- Multi-Rabbit Household
- Weight Entry Detail
- DisplayUnitStore
- Implementation findings — tasks 3.1–3.3 and 5.1–5.5 (2026-10-20)
- Decisions
- Requirements
- ADDED Requirements
- Requirement: Connectivity model: reachability-gated writes, loaded-data reads
- Requirements
- P4 Frontend UI Tasks
- devDependencies
- status.ts
- persist.rs
- PersistedHousehold
- write_path.rs
- super
- ServiceReply
- tests/diagnostics.rs
- sink.rs
- ADDED Requirements
- HouseholdOnboardingScreen.tsx
- setup.ts
- fixtures.ts
- ADDED Requirements
- collection_identity.rs
- cip/generate.py
- Requirement: Display weight unit is per-device and applied at the boundary
- e2e-fixture-consumption.spec.ts
- sync/mod.rs
- vitest
- csv
- test_design_system_mode.py
- TestNativeDesktopStackFreshness
- ring
- http.rs
- bindings.ts
- ADDED Requirements
- settings.rs
- DeviceSnapshot
- SharedService
- Tasks — add-household-sharing-hardening (P3)
- BM25
- Working agreement
- BM25
- TestFixtureValidation
- types
- TestSearchDomains
- TestThresholdGate
- SettingsScreen.tsx
- TestWebStackFreshness
- Requirement: Household-scoped isolation and canonical storage at the boundary
- scripts
- D0.9 — The bindings pipeline ships with an empty command set
- diagnostics/mod.rs
- dependencies
- Requirement: Household-scoped isolation and canonical storage at the boundary
- Design — add-shared-household-foundation (P0)
- household-flow.spec.ts
- TestStyleTaxonomy
- commands/household.rs
- events.rs
- status.ts
- path
- offline-launch.spec.ts
- ShadcnInstaller
- json
- button
- Tasks — add-read-path-performance (P6)
- add-shared-household-foundation
- input
- $type
- radius
- sync.rs
- lg
- sm
- padding-y
- 16
- D0.8 — Scaffold baseline and exact pins (section 0)
- 1
- 3
- 8
- destructive
- destructive-foreground
- Requirement: Continuous integration runs the same gate as the definition of done
- muted
- primary-foreground
- secondary-foreground
- D1.2 Grams are canonical; the display unit exists only at the boundary
- D0.16 — Deployment targets are derived, not chosen (Q5)
- TestDomainDetection
- Window
- D0.11 — The baseline is generated by the official Tauri v2 CLI, then refined
- P1 — MVP core loop
- RAB-2 A rabbit profile requires a non-blank name

## God Nodes (most connected - your core abstractions)
1. `block_on_ready()` - 80 edges
2. `AppState` - 75 edges
3. `ServiceReply` - 67 edges
4. `TailwindConfigGenerator` - 58 edges
5. `ScriptedService` - 54 edges
6. `classNames()` - 45 edges
7. `react` - 39 edges
8. `HouseholdRecord` - 36 edges
9. `TestTailwindConfigGenerator` - 35 edges
10. `DesignSystemGenerator` - 35 edges

## Surprising Connections (you probably didn't know these)
- `Q52 — Tasks 7.7 and 7.9 share one commit, and 7.9's focus claim was corrected` --references--> `Button()`  [INFERRED]
  openspec/changes/archive/2026-10-22-add-shared-household-foundation/questions.md → src/components/ui/Button.tsx
- `Q43 — `src/routes/onboarding.tsx` still imports the placeholder alias` --references--> `HouseholdOnboardingScreen()`  [INFERRED]
  openspec/changes/archive/2026-10-22-add-shared-household-foundation/questions.md → src/features/households/components/HouseholdOnboardingScreen.tsx
- `Q58 — `error.unknown`'s copy is the bare message NFR-8 forbids, and it promises a retry that does not exist` --references--> `SaveFailureNotice()`  [INFERRED]
  openspec/changes/archive/2026-10-22-add-shared-household-foundation/questions.md → src/features/sync/components/SaveFailureNotice.tsx
- `Q21 — the keyboard inset has no owner until the first real form screen (resolved by assignment, 2026-10-19)` --references--> `Screen()`  [INFERRED]
  openspec/changes/archive/2026-10-22-add-shared-household-foundation/questions.md → src/components/ui/Screen.tsx
- `4. Frontend UI` --references--> `Screen()`  [INFERRED]
  openspec/changes/archive/2026-10-22-add-shared-household-foundation/tasks.md → src/components/ui/Screen.tsx

## Import Cycles
- 1-file cycle: `src-tauri/src/ipc/events.rs -> src-tauri/src/ipc/events.rs`
- 2-file cycle: `src-tauri/src/sync/gate.rs -> src-tauri/src/sync/mod.rs -> src-tauri/src/sync/gate.rs`
- 2-file cycle: `src-tauri/src/sync/mod.rs -> src-tauri/src/sync/probe.rs -> src-tauri/src/sync/mod.rs`
- 2-file cycle: `src-tauri/src/sync/mod.rs -> src-tauri/src/sync/status.rs -> src-tauri/src/sync/mod.rs`
- 2-file cycle: `src-tauri/src/settings/region.rs -> src-tauri/src/settings/unit.rs -> src-tauri/src/settings/region.rs`
- 3-file cycle: `src-tauri/src/sync/gate.rs -> src-tauri/src/sync/probe.rs -> src-tauri/src/sync/mod.rs -> src-tauri/src/sync/gate.rs`
- 3-file cycle: `src-tauri/src/sync/mod.rs -> src-tauri/src/sync/status.rs -> src-tauri/src/sync/probe.rs -> src-tauri/src/sync/mod.rs`
- 4-file cycle: `src-tauri/src/state.rs -> src-tauri/src/sync/status.rs -> src-tauri/src/sync/mod.rs -> src-tauri/src/sync/merge.rs -> src-tauri/src/state.rs`
- 5-file cycle: `src-tauri/src/state.rs -> src-tauri/src/sync/status.rs -> src-tauri/src/sync/probe.rs -> src-tauri/src/sync/mod.rs -> src-tauri/src/sync/merge.rs -> src-tauri/src/state.rs`

## Hyperedges (group relationships)
- **Bounded load-once read path** — openspec_changes_add_read_path_performance_design_fixed_two_request_bound, openspec_changes_add_read_path_performance_design_freshness_window, openspec_changes_add_read_path_performance_design_load_once_per_area, openspec_changes_add_read_path_performance_design_scoped_invalidation [EXTRACTED 0.90]
- **Household sharing flows** — openspec_changes_add_household_sharing_hardening_design_one_invitation_reuse, openspec_changes_add_household_sharing_hardening_design_acceptance_adoption, openspec_changes_add_household_sharing_hardening_design_owner_gated_deletion_cascade, openspec_changes_add_household_sharing_hardening_design_local_first_member_list [EXTRACTED 0.90]
- **Near-simultaneous dose guard** — openspec_changes_add_medication_tracking_design_near_simultaneous_guard, openspec_changes_add_medication_tracking_design_entered_timestamp_anchor, openspec_changes_add_medication_tracking_design_fresh_read_guard [EXTRACTED 0.90]
- **Accessibility Pattern Set** — _opencode_skills_ui_styling_skill_radix_ui, _opencode_skills_ui_styling_references_shadcn_accessibility_wai_aria, _opencode_skills_ui_styling_references_shadcn_accessibility_focus_management, _opencode_skills_ui_styling_references_shadcn_accessibility_wcag_contrast, _opencode_skills_ui_styling_references_shadcn_accessibility [EXTRACTED 1.00]
- **ADR 0001 Reimplementation Scope Decisions** — docs_adr_0001_reimplementation_scope_decisions_d1_write_model, docs_adr_0001_reimplementation_scope_decisions_d2_sharing_model, docs_adr_0001_reimplementation_scope_decisions_d3_reminder_unification, docs_adr_0001_reimplementation_scope_decisions_d4_migration_fresh_start, docs_adr_0001_reimplementation_scope_decisions_d5_chart_baseline, docs_adr_0001_reimplementation_scope_decisions_d6_shared_service_deferred [EXTRACTED 1.00]
- **BunScale Agent Routing System** — agents_agent_routing, _opencode_agents_orchestrator_agent, _opencode_agents_spec_writer_agent, _opencode_agents_architect_agent, _opencode_agents_rust_backend_agent, _opencode_agents_apple_native_agent, _opencode_agents_frontend_logic_agent, _opencode_agents_frontend_ui_agent, _opencode_agents_tauri_security_agent, _opencode_agents_test_engineer_agent, _opencode_agents_reviewer_agent, _opencode_agents_release_ci_agent [EXTRACTED 1.00]
- **CIP Design Pipeline** — _opencode_skills_design_skill_cip_design, _opencode_skills_design_references_cip_design, _opencode_skills_design_references_cip_deliverable_guide, _opencode_skills_design_references_cip_style_guide, _opencode_skills_design_references_cip_prompt_engineering [EXTRACTED 1.00]
- **Logo Design Pipeline** — _opencode_skills_design_skill_logo_design, _opencode_skills_design_references_logo_design, _opencode_skills_design_references_logo_style_guide, _opencode_skills_design_references_logo_color_psychology, _opencode_skills_design_references_logo_prompt_engineering [EXTRACTED 1.00]
- **OpenSpec change workflow cycle** — _opencode_commands_opsx_new, _opencode_commands_opsx_propose, _opencode_commands_opsx_apply, _opencode_commands_opsx_archive, _opencode_commands_opsx_verify, _opencode_commands_opsx_continue [EXTRACTED 1.00]
- **shadcn/ui Component Stack** — _opencode_skills_ui_styling_skill_shadcn_ui, _opencode_skills_ui_styling_skill_radix_ui, _opencode_skills_ui_styling_skill_tailwind_css, _opencode_skills_ui_styling_references_shadcn_components [EXTRACTED 1.00]
- **Slides Knowledge Base Flow** — _opencode_skills_design_references_slides, _opencode_skills_design_references_slides_create, _opencode_skills_design_references_slides_layout_patterns, _opencode_skills_design_references_slides_html_template, _opencode_skills_design_references_slides_copywriting_formulas, _opencode_skills_design_references_slides_strategies [EXTRACTED 1.00]
- **Theming and Design Token System** — _opencode_skills_ui_styling_skill_css_variables, _opencode_skills_ui_styling_skill_dark_mode, _opencode_skills_ui_styling_references_tailwind_customization_theme_directive, _opencode_skills_ui_styling_references_shadcn_theming, _opencode_skills_ui_styling_references_tailwind_customization [EXTRACTED 1.00]
- **Three-Layer Token Architecture** — _opencode_skills_design_system_references_token_architecture_token_architecture, _opencode_skills_design_system_references_primitive_tokens_primitive_tokens, _opencode_skills_design_system_references_semantic_tokens_semantic_tokens, _opencode_skills_design_system_references_component_tokens_component_tokens, _opencode_skills_design_system_skill_three_layer_token_structure [EXTRACTED 1.00]
- **Derived weight-drop alert shared by entry and dashboard** — openspec_changes_add_rabbit_weight_core_loop_design_d1_3, openspec_changes_add_rabbit_weight_core_loop_design_d1_12, openspec_changes_add_rabbit_weight_core_loop_design_d1_13, openspec_changes_add_rabbit_weight_core_loop_specs_weight_tracking_spec_wt_7, openspec_changes_add_rabbit_weight_core_loop_specs_dashboard_spec_dash_4 [EXTRACTED 1.00]
- **Sharing Correctness and Write-Model Risk Cluster** — docs_bunscale_reimplementation_requirements_household_sharing, docs_bunscale_reimplementation_requirements_synchronization, docs_bunscale_reimplementation_requirements_offline_write_conflict, docs_bunscale_reimplementation_requirements_last_write_wins, docs_bunscale_reimplementation_user_stories_us_med_23, docs_bunscale_reimplementation_user_stories_delivery_phases [INFERRED 0.75]
- **Background-execution reconciliation and cancellation of follow-ups** — openspec_changes_add_vet_export_and_reminders_design_d4_3_follow_up_evaluation, openspec_changes_add_vet_export_and_reminders_design_d4_5_one_background_mechanism, openspec_changes_add_vet_export_and_reminders_specs_medication_tracking_spec_med_10, openspec_changes_add_vet_export_and_reminders_specs_reminders_spec_rem_3 [INFERRED 0.85]
- **Design System Reference Set** — _opencode_skills_design_system_skill_three_layer_token_structure, _opencode_skills_design_system_references_component_specs_button_spec, _opencode_skills_design_system_references_states_and_variants_interactive_states, _opencode_skills_design_system_references_tailwind_integration_tailwind_integration [INFERRED 0.85]
- **Vet-readable export document generation (PDF and raw CSV)** — openspec_changes_add_vet_export_and_reminders_design_d4_4_document_generation, openspec_changes_add_vet_export_and_reminders_specs_export_spec_exp_1, openspec_changes_add_vet_export_and_reminders_specs_export_spec_exp_2, openspec_changes_add_vet_export_and_reminders_specs_export_spec_exp_4 [INFERRED 0.85]
- **Health logging and history pipeline** — openspec_changes_add_health_and_photo_history_specs_health_logging_spec_health_entry, openspec_changes_add_health_and_photo_history_specs_health_logging_spec_health_severity, openspec_changes_add_health_and_photo_history_specs_health_logging_spec_health_entry_filter, openspec_changes_add_health_and_photo_history_specs_data_integrity_spec_health_tag_catalog, openspec_changes_add_health_and_photo_history_specs_data_integrity_spec_canonical_health_category [INFERRED 0.85]
- **OpenSpec Change Lifecycle** — _opencode_skills_openspec_propose_skill_openspec_propose, _opencode_skills_openspec_explore_skill_openspec_explore, _opencode_skills_openspec_new_change_skill_openspec_new_change, _opencode_skills_openspec_apply_change_skill_openspec_apply_change, _opencode_skills_openspec_verify_change_skill_openspec_verify_change, _opencode_skills_openspec_archive_change_skill_openspec_archive_change [INFERRED 0.85]
- **OpenSpec Store Selection Pattern** — _opencode_skills_openspec_apply_change_skill_store_selection, _opencode_skills_openspec_apply_change_skill_openspec_apply_change, _opencode_skills_openspec_archive_change_skill_openspec_archive_change, _opencode_skills_openspec_continue_change_skill_openspec_continue_change, _opencode_skills_openspec_explore_skill_openspec_explore, _opencode_skills_openspec_new_change_skill_openspec_new_change, _opencode_skills_openspec_onboard_skill_openspec_onboard, _opencode_skills_openspec_propose_skill_openspec_propose, _opencode_skills_openspec_sync_specs_skill_openspec_sync_specs, _opencode_skills_openspec_update_change_skill_openspec_update_change, _opencode_skills_openspec_verify_change_skill_openspec_verify_change [INFERRED 0.85]
- **Photo offline availability flow** — openspec_changes_add_health_and_photo_history_design_d2_2, openspec_changes_add_health_and_photo_history_design_d2_3, openspec_changes_add_health_and_photo_history_design_d2_9, openspec_changes_add_health_and_photo_history_specs_sync_and_offline_spec_viewed_photo_offline, openspec_changes_add_health_and_photo_history_specs_photos_spec_photo_owner [INFERRED 0.85]
- **Slides Deck Composition** — _opencode_skills_slides_skill_slides, _opencode_skills_slides_references_html_template_html_template, _opencode_skills_slides_references_layout_patterns_layout_patterns, _opencode_skills_slides_references_copywriting_formulas_copywriting_formulas, _opencode_skills_slides_references_slide_strategies_slide_strategies, _opencode_skills_slides_skill_design_tokens [INFERRED 0.85]
- **One unified reminder mechanism (D3) spanning weigh and medication reminders** — openspec_changes_add_vet_export_and_reminders_design_d4_1_one_reminder_mechanism, openspec_changes_add_vet_export_and_reminders_specs_reminders_spec_unified_mechanism, openspec_changes_add_vet_export_and_reminders_specs_medication_tracking_spec_med_9, openspec_changes_add_vet_export_and_reminders_specs_reminders_spec_rem_1 [INFERRED 0.85]
- **AppError Contract Chain** — _opencode_skills_tauri_ipc_contract_skill_apperror, _opencode_skills_tauri_ipc_contract_skill_command_rules, _opencode_skills_rust_conventions_skill_error_conventions, _opencode_skills_tauri_apple_plugins_skill_plugin_rules [INFERRED 0.90]

## Communities (225 total, 53 thin omitted)

### Community 0 - "entry.ts"
Cohesion: 0.15
Nodes (18): Q31 — The held-set list and the active household are deliberately not household-keyed, getActiveHousehold(), hasHousehold(), HouseholdEntry, householdEntryDecision(), HouseholdPresence, isFreshReachableAccount(), resolveHouseholdEntry() (+10 more)

### Community 1 - "scripts/core.py"
Cohesion: 0.06
Nodes (31): BM25, _contains_phrase(), detect_domain(), _domain_keywords(), _exact_match_diagnostic(), _exact_row_identity(), _exact_stack_identifier(), _file_signature() (+23 more)

### Community 2 - "gray"
Cohesion: 0.05
Nodes (53): $type, $value, $type, $value, $type, $value, $type, $value (+45 more)

### Community 3 - "design_system.py"
Cohesion: 0.04
Nodes (19): ansi_ljust(), _detect_page_type(), format_ascii_box(), format_markdown(), format_master_md(), format_page_override_md(), generate_design_system(), _generate_intelligent_overrides() (+11 more)

### Community 4 - "P1 IPC contract (proposal, architect-owned)"
Cohesion: 0.12
Nodes (15): DashboardRow shared type, P1 IPC contract (proposal, architect-owned), Rabbit shared type, WeightEntry shared type, WeightTrend shared type, Q3 sex and neuter/spay status value sets, DASH-1 Dashboard lists non-archived rabbits with name, thumbnail and last-weighed date, DASH-6 Selecting a rabbit opens that rabbit's dashboard (+7 more)

### Community 5 - "slide_search_core.py"
Cohesion: 0.08
Nodes (17): format_context(), format_result(), main(), BM25, calculate_pattern_break(), detect_domain(), get_background_config(), get_color_for_emotion() (+9 more)

### Community 6 - "Export & Reminders Decisions"
Cohesion: 0.09
Nodes (24): P4 IPC contract (reminder, export, language commands and shared types), Reminder occurrence model (pending|fired|snoozed|satisfied|missed), P4 risk register (R1–R13), Phase P4 — Handoff & preferences, P4 settled questions (former Q1–Q6), EXP-1 — History exports as a formatted, vet-readable PDF for a selected range, EXP-2 — Same history exports as raw CSV with dedicated medication and dose files, EXP-3 — Exported files handed to the device's standard sharing options (+16 more)

### Community 7 - "DM-4 An archived rabbit is read-only everywhere"
Cohesion: 0.25
Nodes (7): DM-4 An archived rabbit is read-only everywhere, RAB-7 An archived rabbit is restorable with its history intact, RAB-8 An archived rabbit's timeline is read-only, RAB-9 Archived rabbit data is retained by default, WT-5 A past weight entry is editable and deletable and the change is shared, Task 2.8 Enforce archived-rabbit immutability on every write path, Task 6.1 Verify the delta contains exactly the 30 requirement IDs

### Community 8 - "Health & Photos Design"
Cohesion: 0.08
Nodes (24): Design — add-health-and-photo-history (P2), P2 IPC contract (proposed commands, events, shared types), Proposal — add-health-and-photo-history, P2 — Health & evidence phase, Questions — add-health-and-photo-history (P2), dashboard delta spec (DASH-4, DASH-7, HL-3), Dashboard severe-health flag (DASH-4 severe clause), History views (weight-only | health-only | combined) (+16 more)

### Community 9 - "commands/identity.rs"
Cohesion: 0.05
Nodes (56): a_blank_name_is_absent_and_the_chain_falls_through(), a_local_name_without_an_account_is_not_attributable_and_the_identity_is_unresolved(), a_name_is_never_taken_from_another_account(), account(), an_account_name_without_an_account_is_not_used(), an_unresolved_identity_is_the_marker_and_not_a_failure(), local_member_name(), member() (+48 more)

### Community 10 - "Design Skill References"
Cohesion: 0.11
Nodes (34): Banner Sizes & Art Direction Styles Reference, 22 Art Direction Styles, CIP Deliverable Guide, CIP Design Reference, CIP Mockup Prompt Engineering, CIP Base Prompt Structure, CIP Design Style Guide, CIP Color Psychology (+26 more)

### Community 11 - "local.rs"
Cohesion: 0.08
Nodes (42): a_collection_carries_the_household_it_belongs_to(), a_new_collection_holds_nothing(), a_removal_is_a_tombstone_rather_than_a_vanished_row(), a_second_load_of_the_same_batch_does_not_duplicate_an_item(), a_second_upsert_of_the_same_item_keeps_exactly_one_copy(), a_wire_item_round_trips_through_the_stored_row_unchanged(), at(), CollectionItem (+34 more)

### Community 12 - "html-token-validator.py"
Cohesion: 0.13
Nodes (12): get_context(), is_allowed_exception(), is_allowed_rgba(), is_inside_block(), load_css_variables(), main(), print_result(), print_summary() (+4 more)

### Community 13 - "Reimplementation Domain Model"
Cohesion: 0.06
Nodes (52): Apple-Native Agent, Custom Swift Tauri Plugin, Architect Agent, IPC Contract (Commands, Events, Types), Frontend-Logic Agent, Typed IPC Wrapper (api.ts), Frontend-UI Agent, Design System Bootstrap (+44 more)

### Community 14 - "Component Token Values"
Cohesion: 0.09
Nodes (22): $type, $value, $type, $value, $type, $value, $type, $value (+14 more)

### Community 15 - "status.rs"
Cohesion: 0.18
Nodes (16): AccountOutcome, Identified, NoAccount, RateLimited, Unreadable, a_success_between_failures_resets_the_count(), account_valid(), an_account_check_that_gets_no_answer_counts_towards_the_threshold() (+8 more)

### Community 16 - "Icon Generation Pipeline"
Cohesion: 0.11
Nodes (13): apply_color(), apply_viewbox_size(), extract_svgs(), generate_batch(), generate_icon(), generate_sizes(), load_env(), main() (+5 more)

### Community 17 - "UI Styling Concepts"
Cohesion: 0.11
Nodes (24): Apache License 2.0, Canvas Design System, oklch Color Space, shadcn/ui Accessibility Patterns, Focus Management, WAI-ARIA Design Patterns, WCAG Color Contrast, shadcn/ui Component Reference (+16 more)

### Community 18 - "multiple_households.rs"
Cohesion: 0.15
Nodes (21): a_second_household_coexists_without_mixing_data(), accepted_answer(), account(), active_household_is_replaced_when_it_is_no_longer_held(), active_rows(), at(), collection(), create() (+13 more)

### Community 19 - "OpenSpec Workflow Skills"
Cohesion: 0.15
Nodes (20): OpenSpec Apply Change Skill, OpenSpec Store Selection, OpenSpec Archive Change Skill, OpenSpec Continue Change Skill, Explore Mode, OpenSpec Explore Skill, OpenSpec New Change Skill, OpenSpec Onboard Skill (+12 more)

### Community 20 - "read_path.rs"
Cohesion: 0.26
Nodes (14): a_failed_refresh_leaves_the_previous_copy_exactly_as_it_was(), a_start_with_connectivity_refreshes_loaded_data(), at(), collection(), feed(), household_id(), item(), item_id() (+6 more)

### Community 21 - "queries.ts"
Cohesion: 0.04
Nodes (60): @tauri-apps/api, makeEvent(), AVAILABLE, HOUSEHOLD, SUMMARY, UNOBSERVED, HouseholdChange, loadHousehold() (+52 more)

### Community 22 - "storage_boundary.rs"
Cohesion: 0.11
Nodes (19): CanonicalForm, .FORM, a_removal_keeps_its_households_claim(), Amount, .FORM, at(), collection(), household_id() (+11 more)

### Community 23 - "Open items"
Cohesion: 0.05
Nodes (42): 6.2 review — APPROVE (2026-10-22), Gate — all green, History — how Q10–Q12 were raised (2026-10-16, orchestrator), iOS — iOS 27.0 simulator (24A434), iPhone 18 Pro, macOS — macOS 27.0.1 (26A434), Open items, Q10 — decided: fix the harness configuration first, then delegate, Q10 — Specialists could not execute shell commands in this harness (+34 more)

### Community 24 - "failure_surface.rs"
Cohesion: 0.09
Nodes (14): a_failed_save_is_visible_and_specific(), account(), assert_check_service_reachability_shape(), assert_get_active_household_shape(), assert_get_identity_shape(), assert_get_service_status_shape(), assert_list_households_shape(), create_failure() (+6 more)

### Community 26 - "Sharing Runtime Behavior"
Cohesion: 0.11
Nodes (16): Live updates and background delivery handoff (D-H6), Status derived from fresh outcomes (D-H8), NotificationAuthorization state, Set-up-again recovery from shared service (D-H9), SharingPolicy configuration values, Visible household-naming notification (D-H14), HS-11 Changes reach every member without manual refresh, NFR-6 Honest service status (+8 more)

### Community 27 - "lifecycle.rs"
Cohesion: 0.08
Nodes (32): a_probe_that_gets_no_answer_counts_towards_the_threshold(), a_re_evaluation_probes_with_the_reachability_bound(), a_re_evaluation_publishes_the_fresh_status_it_returns(), a_trigger_during_an_in_flight_re_evaluation_runs_exactly_one_follow_up(), a_trigger_whose_probe_gets_no_answer_publishes_no_refresh_for_a_retained_status(), a_trigger_with_a_reachable_service_and_an_active_household_publishes_one_refreshed(), a_trigger_with_an_invalid_account_publishes_no_household_change(), a_trigger_with_no_active_household_publishes_no_household_change() (+24 more)

### Community 28 - "color"
Cohesion: 0.11
Nodes (19): $type, $value, background, foreground, muted-foreground, primary, primary-hover, secondary (+11 more)

### Community 29 - "Sharing Delivery Ordering"
Cohesion: 0.11
Nodes (17): Offline member deletion window (D-H15), Owner-gated irreversible deletion cascade (D-H5), Archive order P0-P1-P2-P3-P5-P4-P6, HS-10 Only the owner can permanently delete a household, Active/finished derived from end date (D5.2), Derived affordance set for read-only states (D5.6), Enumerated units with no conversion (D5.7), Dose giver captured from recording member (D5.4, DM-5) (+9 more)

### Community 30 - "Household Invitation Design"
Cohesion: 0.12
Nodes (13): Invitation acceptance as household adoption (D-H3), Member cap enforced at membership level (D-H13), One invitation per household, resolved by reuse (D-H2), Client-generated request_id idempotency, Convergence as primary risk (R1), Settled decisions D1-D6, HS-5 Inviting is availability-checked and never duplicated, HS-6 Accepting an invitation adopts the household (+5 more)

### Community 31 - "Design Token Layers"
Cohesion: 0.17
Nodes (10): Table Component Spec, Component Tokens Layer, Primitive Color Scales, Primitive Tokens, 4px Spacing Scale, Typography Scale, Semantic Tokens, shadcn/ui Alignment (+2 more)

### Community 32 - "guards.rs"
Cohesion: 0.11
Nodes (41): a_batch_containing_a_foreign_item_is_refused_whole(), a_guarded_load_carrying_a_marked_row_unchanged_succeeds_and_records_the_marker(), a_guarded_load_containing_another_households_item_is_refused_whole(), a_guarded_load_describing_another_household_is_refused(), a_guarded_load_of_an_empty_collection_is_a_loaded_empty_copy(), a_guarded_load_records_the_load_instant_beside_the_rows(), a_guarded_load_that_would_omit_a_marked_entity_is_refused_and_changes_nothing(), a_guarded_load_that_would_rewrite_a_marked_entity_is_refused_and_changes_nothing() (+33 more)

### Community 33 - "fontSize"
Cohesion: 0.12
Nodes (16): $type, $value, $type, $value, $type, $value, $type, $value (+8 more)

### Community 35 - "Slides Copywriting & Template"
Cohesion: 0.15
Nodes (15): AIDA Copywriting Formula, Copywriting Formulas Reference, FAB Copywriting Formula, PAS Copywriting Formula, Slides Create Subcommand, Chart.js Integration, HTML Slide Template Reference, Feature Grid Layout (+7 more)

### Community 36 - "Brand Color Extraction"
Cohesion: 0.20
Nodes (11): calculateCompliance(), colorDistance(), displayPalette(), extractHexColors(), findNearestBrandColor(), fs, generateImageMagickCommand(), hexToRgb() (+3 more)

### Community 37 - "identity_and_unit.rs"
Cohesion: 0.11
Nodes (9): account(), default_unit_follows_the_device_region(), household_id(), remove_temp_data_dir(), resolved_account_name_is_shown_read_only(), shipped_commands(), temp_data_dir(), unit_choice_is_stored_on_the_device() (+1 more)

### Community 38 - "biome.json"
Cohesion: 0.08
Nodes (24): source, assist, actions, enabled, files, includes, formatter, enabled (+16 more)

### Community 40 - "OpenSpec CLI Commands"
Cohesion: 0.33
Nodes (10): opsx-apply command, opsx-archive command, opsx-continue command, opsx-explore command, opsx-new command, opsx-onboard command, opsx-propose command, opsx-sync command (+2 more)

### Community 41 - "Brand Asset Validation"
Cohesion: 0.25
Nodes (13): checkManifest(), formatBytes(), formatOutput(), fs, main(), parseFilename(), path, RULES (+5 more)

### Community 42 - "conflict_convergence.rs"
Cohesion: 0.09
Nodes (36): a_supplied_instant_is_never_rounded_or_clamped(), a_supplied_past_instant_survives_creation_unchanged(), AbsoluteInstant, an_instant_whose_utc_year_exceeds_the_rfc_3339_range_is_refused(), an_instant_whose_utc_year_precedes_the_rfc_3339_range_is_refused(), an_invalid_supplied_instant_is_refused_rather_than_replaced_with_now(), creation_without_a_supplied_instant_defaults_to_the_creation_moment(), instant() (+28 more)

### Community 44 - "desktop-settings.spec.ts"
Cohesion: 0.13
Nodes (10): dashboard(), openDashboardWithTwoHouseholds(), openSettingsFromDashboard(), RecorderWindow, screenTitle(), SeededHouseholds, seedHouseholds(), SeedWindow (+2 more)

### Community 45 - "Brand Guidelines References"
Cohesion: 0.21
Nodes (13): Asset Approval Checklist, Asset Organization Guide, Brand Guidelines Template, Color Palette Management, Brand Consistency Checklist, Logo Usage Rules, Messaging Framework, Typography Specifications (+5 more)

### Community 46 - "tauri.conf.json"
Cohesion: 0.06
Nodes (31): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+23 more)

### Community 48 - "compilerOptions"
Cohesion: 0.10
Nodes (20): compilerOptions, esModuleInterop, isolatedModules, jsx, lib, module, moduleDetection, moduleResolution (+12 more)

### Community 49 - "timestamps.rs"
Cohesion: 0.12
Nodes (23): a_supplied_instant_is_stored_unchanged(), a_timestamp_defaults_to_now(), accepted_answer(), account(), at(), collection(), EPOCH, feed() (+15 more)

### Community 50 - "DesignSystemGenerator"
Cohesion: 0.06
Nodes (12): DesignSystemGenerator, apply_decision_rules(), _object_without_duplicates(), parse_decision_rules(), _validate_action(), TestReasoningMatch, read_rows(), split_values() (+4 more)

### Community 51 - "json"
Cohesion: 0.08
Nodes (14): format_brief(), format_results(), main(), format_output(), generate_design_brief(), generate_css_for_background(), get_background_image(), get_curated_images() (+6 more)

### Community 52 - "read.rs"
Cohesion: 0.15
Nodes (35): ItemId, block_on_ready(), a_blank_household_id_is_refused_as_invalid_input(), a_corrupt_or_absent_store_degrades_to_never_loaded(), a_failed_persistence_write_does_not_fail_the_load(), a_failed_refresh_leaves_the_loaded_copy_exactly_as_it_was(), a_feed_describing_another_household_does_not_replace_the_local_copy(), a_full_answer_that_omits_an_item_replaces_the_copy() (+27 more)

### Community 53 - "Embed Tokens Script"
Cohesion: 0.17
Nodes (8): args, fs, minimal, MINIMAL_TOKENS, path, projectRoot, tokensPath, wrapStyle

### Community 54 - "Token Validator Script"
Cohesion: 0.24
Nodes (11): extensions, formatReport(), fs, getFiles(), main(), parseArgs(), path, patterns (+3 more)

### Community 55 - "card"
Cohesion: 0.20
Nodes (12): $type, $value, bg, bg, padding, shadow, card, bg (+4 more)

### Community 56 - "css-invariants.ts"
Cohesion: 0.13
Nodes (25): checkTokenInvariants(), closingBrace(), ContrastPair, contrastRatio(), createThemeResolver(), crossContextRedefinitions(), CssDeclaration, CssReference (+17 more)

### Community 57 - "Brand Context Injection"
Cohesion: 0.31
Nodes (10): extractColorsFromTable(), extractCoreAttributes(), extractHexColors(), extractImageStyle(), extractTypography(), extractVoice(), fs, generatePromptAddition() (+2 more)

### Community 58 - "Motion Token Values"
Cohesion: 0.20
Nodes (10): fast, normal, slow, $type, $value, $type, $value, duration (+2 more)

### Community 60 - "package.json"
Cohesion: 0.12
Nodes (16): name, private, type, @biomejs/biome, jsdom, react-dom, @tanstack/router-plugin, @tauri-apps/cli (+8 more)

### Community 62 - "Brand Token Sync Script"
Cohesion: 0.29
Nodes (8): adjustBrightness(), { execFileSync }, extractColorsFromMarkdown(), fs, generateColorScale(), main(), path, updateDesignTokens()

### Community 63 - "Generate Tokens Script"
Cohesion: 0.36
Nodes (9): flattenTokens(), fs, generateCSS(), generateTailwind(), main(), parseArgs(), path, resolveReference() (+1 more)

### Community 64 - "@tanstack/react-router"
Cohesion: 0.07
Nodes (37): @tanstack/react-query, @tanstack/react-router, AppShell(), AppShellProps, DataAttributes, NavigationArrangement, NavigationDestination, Sidebar() (+29 more)

### Community 65 - "scaffold-files.ts"
Cohesion: 0.20
Nodes (12): bindingsFile, FORBIDDEN_NAMES, forbiddenNamesInCode(), rawStringHashes(), stripRustComments(), CommandResult, describeCommand(), readRepositoryFile() (+4 more)

### Community 66 - "Component Specs & States"
Cohesion: 0.22
Nodes (9): UI Task Additional Checks, Button Component Spec, Input Component Spec, Button Tokens, State Accessibility Requirements, Focus Ring, Interactive States, Icon Usage Rules (+1 more)

### Community 67 - "Rust & Tauri Conventions"
Cohesion: 0.22
Nodes (9): Tauri Event Subscription Pattern, Rust Async Conventions, Rust Module Structure, Apple Plugin Structure, Tauri Command Rules, Tauri Event Rules, Native Automation Limits, IPC Mocking (+1 more)

### Community 68 - "design-tokens-starter.json"
Cohesion: 0.15
Nodes (12): component, $type, $value, dark, semantic, $schema, $type, $value (+4 more)

### Community 69 - "SETUP.md — local development setup"
Cohesion: 0.22
Nodes (8): 1. Xcode, 2. Rust and the iOS target, 3. bun, 4. Project dependencies, 5. Tauri CLI, 6. Verify the setup, 7. Running the app, SETUP.md — local development setup

### Community 70 - "Weight Drop Alert Rules"
Cohesion: 0.28
Nodes (6): Q1 which entry is the previous entry on a shared timestamp, Q2 the drop alert as a pure derivation over the current entry set, DASH-4 (weight part) Rabbits with a weight-drop alert are flagged on the dashboard, RAB-5 Every field of an active rabbit's profile is editable and shared, WT-7 A weight drop beyond the threshold raises an alert and flags the rabbit, Task 2.13 Implement the drop-alert derivation

### Community 71 - "probe.rs"
Cohesion: 0.11
Nodes (13): REACHABILITY_PROBE_TIMEOUT, ALL_OUTCOMES, an_account_check_that_gets_no_answer_is_reported_as_its_own_outcome(), probe(), probe_answered(), ProbeOutcome, AccountUnanswered, Answered (+5 more)

### Community 72 - "Requirements"
Cohesion: 0.05
Nodes (36): project-toolchain Specification, Purpose, Requirement: Bindings are generated reproducibly and drift fails the build, Requirement: Continuous integration runs the same gate as the definition of done, Requirement: The app boots on the iOS simulator and on macOS, Requirement: The design system is established and its status recorded before dependent UI work, Requirement: The route tree is generated reproducibly and drift fails the build, Requirement: The test harness runs at both form-factor baselines (+28 more)

### Community 75 - "validate_data.py"
Cohesion: 0.06
Nodes (44): read_rows(), TestAccessibilityGuidance, TestChartsTypographyAndIcons, TestCurrentReactGuidance, TestSemanticColors, TestGeneratedCatalogContract, _catalog_date(), _check_app_interface_contract() (+36 more)

### Community 76 - "store.rs"
Cohesion: 0.26
Nodes (22): a_choice_is_still_in_effect_for_a_store_that_reads_it_again(), a_device_with_no_file_has_no_choice_and_that_is_not_a_failure(), a_file_a_later_version_extended_still_reads_this_preference(), a_file_this_version_did_not_write_is_a_failure_rather_than_a_default(), a_first_write_creates_the_data_directory(), a_malformed_file_does_not_block_storing_a_choice(), a_stored_choice_is_honoured_without_reading_the_region(), a_stored_choice_is_the_effective_unit_whatever_the_region_states() (+14 more)

### Community 78 - "default.json"
Cohesion: 0.29
Nodes (6): description, identifier, permissions, platforms, $schema, windows

### Community 79 - "ADR 0002 — Shared service mechanism: where the shared service runs and how it is written to and read from"
Cohesion: 0.22
Nodes (8): ADR 0002 — Shared service mechanism: where the shared service runs and how it is written to and read from, Alternatives considered, Consequences, Context, Decision, M1 — Selected mechanism: the self-hosted, product-operated Supabase stack, R1 — Loaded-data read model (implements SYNC-3; `design.md` D0.2), W1 — Connectivity-required write model (implements ADR 0001 D1; `design.md` D0.1)

### Community 80 - "Release & Definition of Done"
Cohesion: 0.33
Nodes (6): Definition of Done, Gate Command Sequence, iOS Signing and TestFlight Delivery, macOS Distribution Decision, Release CI Pipeline, Updater Security Rules

### Community 81 - "Tauri Security Rules"
Cohesion: 0.40
Nodes (6): Rust Platform Gating, Apple Plugin Rules, Tauri Capabilities, Content Security Policy, Per-platform Security Constraints, Tauri Scopes

### Community 82 - "radius"
Cohesion: 0.19
Nodes (14): $type, $value, $type, $value, $type, $value, primitive, radius (+6 more)

### Community 83 - "merge.rs"
Cohesion: 0.18
Nodes (30): a_batch_that_mixes_winners_and_losers_reports_each_item_once(), a_collection_describing_another_household_is_refused_whole(), a_merge_leaves_the_loaded_at_marker_alone(), a_merge_requires_a_loaded_copy_and_never_creates_one(), a_newer_version_replaces_the_stored_row(), a_stored_unparseable_version_yields_to_a_parseable_delivered_one(), a_write_to_an_immutable_item_is_refused_and_changes_nothing(), an_equal_instant_duplicate_in_one_batch_keeps_the_last_supplied_row() (+22 more)

### Community 84 - "records.rs"
Cohesion: 0.15
Nodes (19): at(), DECLARED_KEYS, DiagnosticRecord, DiagnosticSubject, Household, Request, every_constructor_records_the_outcome_and_identifier_it_names(), every_constructor_serialises_exactly_its_declared_keys() (+11 more)

### Community 85 - "Frontend IPC & Errors"
Cohesion: 0.40
Nodes (5): Feature Folder Structure, Frontend IPC Rules, Rust Error Conventions, AppError, tauri-specta Typed Bindings

### Community 86 - "Mobile Router & History"
Cohesion: 0.40
Nodes (5): Mobile-first CSS Rules, Platform Handling, Adaptive Router Layout, Hash History Setup, Test Viewports

### Community 87 - "AppState"
Cohesion: 0.17
Nodes (14): a_blank_name_is_refused_before_the_account_is_checked(), a_substituted_double_drives_the_real_create_path(), accepted_answer(), account(), at(), household(), no_account_token(), request_id() (+6 more)

### Community 89 - "ADDED Requirements"
Cohesion: 0.07
Nodes (29): ADDED Requirements, Requirement: Bindings are generated reproducibly and drift fails the build, Requirement: The app boots on the iOS simulator and on macOS, Requirement: The design system is established and its status recorded before dependent UI work, Requirement: The route tree is generated reproducibly and drift fails the build, Requirement: The test harness runs at both form-factor baselines, Requirement: The toolchain is pinned exactly and supports both platforms, Requirement: Workspace builds from a clean checkout and the full gate passes (+21 more)

### Community 90 - "gate.rs"
Cohesion: 0.14
Nodes (27): a_2xx_the_boundary_cannot_read_is_never_a_completed_write(), a_rate_limited_account_check_refuses_the_write_before_it_is_sent(), a_reissued_write_is_a_completed_write_and_not_a_second_one(), a_save_the_service_accepted_reports_the_completed_write(), a_save_whose_probe_times_out_is_refused_with_timeout(), a_save_without_a_reachable_service_is_refused_with_connectivity(), a_save_without_a_valid_account_is_refused_with_no_account(), a_write_that_gets_no_outcome_in_the_bound_is_refused_with_timeout() (+19 more)

### Community 92 - "Weight Validation Rules"
Cohesion: 0.40
Nodes (3): NFR-9 Primary and destructive actions are visible and confirmed, RAB-10 An archived rabbit and its data are permanently deletable on confirmation, WT-4 Non-positive values are rejected and implausible outliers are confirmed

### Community 93 - "Frontend State & Routing"
Cohesion: 0.50
Nodes (4): Lifecycle and Connectivity Handling, Frontend State Management Matrix, Router Data Loading, Typed Search Params

### Community 94 - "Design Dials & Generation"
Cohesion: 0.50
Nodes (3): Design Dials, AI Design System Generation, ui-ux-pro-max Query Contract

### Community 95 - "argparse"
Cohesion: 0.08
Nodes (15): generate_html(), get_deliverable_info(), get_image_base64(), main(), _e(), generate_chart_slide(), generate_cta_slide(), generate_deck() (+7 more)

### Community 96 - "save-failure.spec.ts"
Cohesion: 0.11
Nodes (17): ALLOWANCE_CAUSE_FAILURE, CONNECTIVITY_CAUSE_FAILURE, CreateAttempt, CreateFailure, CreateHandler, CreateHouseholdArgs, entryScreen(), expectNotSaved() (+9 more)

### Community 97 - "create.rs"
Cohesion: 0.06
Nodes (52): a_blank_name_is_refused_before_the_gate_is_called(), a_created_household_is_the_only_active_one(), a_gate_refusal_is_reported_unchanged(), a_replayed_request_id_returns_the_existing_household_and_never_a_second_one(), accepted_answer(), account(), an_accepted_answer_without_a_readable_household_is_never_a_success(), assigned_household() (+44 more)

### Community 98 - "xl"
Cohesion: 0.67
Nodes (4): xl, xl, $type, $value

### Community 99 - "manifest-pins.ts"
Cohesion: 0.18
Nodes (17): backticked(), CARGO_SECTIONS, checkCargoPins(), checkNpmPins(), collectCargoPins(), collectNpmPins(), DesignPinTables, duplicates() (+9 more)

### Community 100 - "Owner Attribution Sharing"
Cohesion: 0.50
Nodes (4): Owner attribution as hard selection criterion (D-H16), Owner-attributed storage allowance (D-H7), No open questions (P3), HS-12 Shared content counts against the owner's allowance

### Community 102 - "region.rs"
Cohesion: 0.15
Nodes (21): a_locale_without_a_territory_states_no_system(), a_metric_territory_states_the_metric_system(), a_territory_less_environment_value_does_not_shadow_a_readable_platform_region(), default_system_locale(), environment_locale(), measurement_system(), measurement_system_from_locale(), measurement_system_from_sources() (+13 more)

### Community 104 - "UI Styling Dependencies"
Cohesion: 0.67
Nodes (3): UI Styling Skill Python Dependencies, pytest, UI Styling Skill Test Requirements

### Community 107 - "none"
Cohesion: 0.67
Nodes (4): $type, $value, none, none

### Community 108 - "AppState"
Cohesion: 0.14
Nodes (8): a_later_snapshot_wins_over_an_older_in_flight_write(), a_merge_schedules_the_merged_copy_for_persistence(), AppState, lock(), remove_temp_data_dir(), restored_copies(), temp_data_dir(), wire_collection()

### Community 109 - "household_creation.rs"
Cohesion: 0.17
Nodes (14): a_name_with_surrounding_whitespace_is_created_and_kept_as_supplied(), a_replayed_create_request_returns_the_existing_household(), accepted_answer(), account(), blank_name_is_rejected(), create(), create_the_first_household(), CREATED_AT (+6 more)

### Community 110 - "service.rs"
Cohesion: 0.10
Nodes (11): a_well_formed_write_keeps_its_identity_function_and_body(), a_write_with_a_blank_identity_or_function_is_refused_as_invalid_input(), Answer, HTTP_BAD_REQUEST, HTTP_UNPROCESSABLE_CONTENT, only_an_answered_call_has_an_answer(), OUTCOME_ALLOWANCE_EXCEEDED, OUTCOME_APPLIED (+3 more)

### Community 119 - "failure.rs"
Cohesion: 0.20
Nodes (9): a_stub_failure_names_the_placeholder_key_and_a_classified_unknown_does_not(), ALL_CAUSES, CausePolicy, CommandError, every_cause_reports_through_its_own_message_key(), every_named_constructor_applies_its_own_causes_policy(), named_constructor(), NOT_IMPLEMENTED_MESSAGE_KEY (+1 more)

### Community 127 - "DisplayUnitStore"
Cohesion: 0.16
Nodes (11): choose(), decode(), DisplayUnitStore, effective(), encode(), is_older_than(), names_a_temporary(), PreferenceError (+3 more)

### Community 128 - "Implementation findings — tasks 3.1–3.3 and 5.1–5.5 (2026-10-20)"
Cohesion: 0.09
Nodes (22): Implementation findings — tasks 3.1–3.3 and 5.1–5.5 (2026-10-20), Q29 — The request-id retry invariant has no unit spec of its own, Q30 — `get_diagnostics_summary` lives in `settings/api.ts`, Q32 — Wall-clock resolution of `SAVE_ATTEMPT_TIMEOUT` is not proved by any test, Q33 — The relaunch read path is only proved by the read module's own unit tests, Q34 — `/` is now the household entry; the preview moved to `/design-preview`, Q35 — One commit carried two test tasks, Q36 — E2E remediation rode with task 3.3 (+14 more)

### Community 129 - "Decisions"
Cohesion: 0.11
Nodes (19): D0.10 — Design-system status convention, D0.12 — An exhausted allowance must be reportable as its own condition (Q1), D0.13 — Concrete bounded times, as named constants (Q2), D0.14 — Client-generated request ids, deduplicated by the service (Q3), D0.15 — App identity (Q4), D0.17 — Biome, for lint and format (Q6), D0.18 — The `tauri-specta` release candidate is pinned, with `specta` (Q7), D0.19 — A branded placeholder icon, generated after the design tokens exist (Q8) (+11 more)

### Community 130 - "Requirements"
Cohesion: 0.11
Nodes (18): household-sharing Specification, Purpose, Requirement: Create a household with a non-blank name, Requirement: Exactly one active household per person, Requirement: First-run household creation onboarding runs once, Requirement: Household management from Settings & Profile, Requirement: Multiple households with exactly one active, Requirements (+10 more)

### Community 131 - "ADDED Requirements"
Cohesion: 0.11
Nodes (17): ADDED Requirements, household-sharing — delta for add-shared-household-foundation, Requirement: Create a household with a non-blank name, Requirement: Exactly one active household per person, Requirement: First-run household creation onboarding runs once, Requirement: Household management from Settings & Profile, Requirement: Multiple households with exactly one active, Scenario: A second household coexists without mixing data (+9 more)

### Community 132 - "Requirement: Connectivity model: reachability-gated writes, loaded-data reads"
Cohesion: 0.11
Nodes (17): product-quality Specification, Purpose, Requirement: Conflict resolution never fails and never duplicates, Requirement: Connectivity model: reachability-gated writes, loaded-data reads, Requirement: Every failure path states its cause, Requirement: Health and weight data stay private; diagnostics exclude content, Requirements, Scenario: A failed save is visible and specific (+9 more)

### Community 133 - "Requirements"
Cohesion: 0.11
Nodes (17): Purpose, Requirement: Concurrent edits converge last-write-wins, Requirement: One logical collection per household, Requirement: Reads use previously loaded data when the service is unreachable, Requirement: Saves require the shared service and never pretend to succeed, Requirements, Scenario: A start with connectivity refreshes loaded data, Scenario: Convergence does not duplicate or fail (+9 more)

### Community 135 - "devDependencies"
Cohesion: 0.12
Nodes (17): devDependencies, @biomejs/biome, jsdom, @playwright/test, @tanstack/router-plugin, @tauri-apps/cli, @testing-library/dom, @testing-library/jest-dom (+9 more)

### Community 136 - "status.ts"
Cohesion: 0.09
Nodes (35): Q56 — `household:changed` is in the contract, the bindings and the consumer, and has no producer, Q57 — the foreground-return refresh is documented and unwired, Q58 — `error.unknown`'s copy is the bare message NFR-8 forbids, and it promises a retry that does not exist, Q59 — two review minors, Review findings — task 6.1 (2026-10-22, reviewer), observe(), release(), useAppEvents() (+27 more)

### Community 138 - "persist.rs"
Cohesion: 0.18
Nodes (21): a_corrupt_file_is_a_failure_rather_than_an_empty_device(), a_file_with_a_blank_identifier_is_malformed(), a_file_with_a_non_absolute_instant_is_malformed(), a_loaded_copy_round_trips_through_the_file(), a_never_loaded_collection_is_not_persisted(), a_second_store_over_the_same_directory_reads_what_the_first_wrote(), a_store_with_no_file_holds_no_copy_and_that_is_not_a_failure(), a_write_leaves_a_fresh_temporary_alone() (+13 more)

### Community 139 - "PersistedHousehold"
Cohesion: 0.17
Nodes (11): persisted_household(), at(), decode(), item(), loaded(), persisted_household(), PersistedCollection, PersistedHousehold (+3 more)

### Community 140 - "write_path.rs"
Cohesion: 0.15
Nodes (22): a_stalled_attempt_is_refused_with_timeout_and_carries_the_documented_bound(), an_intervening_success_resets_the_offline_count(), assert_refused(), at(), collection(), envelope(), healthy(), household_id() (+14 more)

### Community 141 - "super"
Cohesion: 0.17
Nodes (10): CANONICAL_UNIT, effective_unit(), entry_and_display_are_one_value_preserving_pair(), GRAMS_PER_KILOGRAM, METRIC_EVERYDAY_UNIT, regional_unit(), to_canonical(), to_display() (+2 more)

### Community 142 - "ServiceReply"
Cohesion: 0.09
Nodes (18): completed_write_reply(), DoNothing, empty_read_reply(), envelope(), healthy_reply(), next(), RecordedCall, CheckAccount (+10 more)

### Community 143 - "tests/diagnostics.rs"
Cohesion: 0.15
Nodes (18): a_health_payload_shaped_save_failure_leaves_only_counts_and_timestamps(), ANALYTICS_TRANSPORTS, assert_payload_carries_content(), at(), content_values(), DECLARED_SAVE_FAILURE_KEYS, diagnostics_carry_no_content(), health_payload() (+10 more)

### Community 144 - "sink.rs"
Cohesion: 0.12
Nodes (16): a_full_sink_drops_its_oldest_record_rather_than_growing(), a_save_failure_records_the_declared_keys_and_its_cause(), a_sink_of_capacity_zero_holds_nothing(), an_empty_sink_summarises_to_no_records_and_no_timestamps(), at(), DiagnosticSink, probed(), record() (+8 more)

### Community 145 - "ADDED Requirements"
Cohesion: 0.12
Nodes (16): ADDED Requirements, product-quality — delta for add-shared-household-foundation, Requirement: Conflict resolution never fails and never duplicates, Requirement: Connectivity model: reachability-gated writes, loaded-data reads, Requirement: Every failure path states its cause, Requirement: Health and weight data stay private; diagnostics exclude content, Scenario: A failed save is visible and specific, Scenario: Concurrent edits resolve without failing (+8 more)

### Community 146 - "HouseholdOnboardingScreen.tsx"
Cohesion: 0.07
Nodes (50): Q49 — The save-retry store is built and tested but intentionally unmounted, createHousehold(), Attempt, CreateOperation, HouseholdOnboardingScreen(), retryHandle(), run(), submit() (+42 more)

### Community 147 - "setup.ts"
Cohesion: 0.04
Nodes (67): @testing-library/react, @testing-library/user-event, PREVIEW_SECTIONS, renderPreviewScreen(), HOUSEHOLD, renderUnmatchedRoute(), SUMMARY, listHouseholds() (+59 more)

### Community 148 - "fixtures.ts"
Cohesion: 0.14
Nodes (13): E2EFixtures, invokeIsMocked(), stubCommandResult(), TauriInternals, test, PROJECT_VIEWPORTS, HOUSEHOLD, HOUSEHOLD_SUMMARY (+5 more)

### Community 149 - "ADDED Requirements"
Cohesion: 0.12
Nodes (16): ADDED Requirements, Requirement: Concurrent edits converge last-write-wins, Requirement: One logical collection per household, Requirement: Reads use previously loaded data when the service is unreachable, Requirement: Saves require the shared service and never pretend to succeed, Scenario: A start with connectivity refreshes loaded data, Scenario: Convergence does not duplicate or fail, Scenario: Every member resolves the same items (+8 more)

### Community 150 - "collection_identity.rs"
Cohesion: 0.17
Nodes (22): a_reissued_write_adds_no_second_item(), assert_one_copy_per_item(), at(), collection(), completed_answer(), every_member_resolves_the_same_items(), feed(), FIRST_VERSION (+14 more)

### Community 151 - "cip/generate.py"
Cohesion: 0.19
Nodes (7): build_cip_prompt(), check_logo_required(), generate_cip_set(), generate_with_nano_banana(), load_env(), load_logo_image(), main()

### Community 152 - "Requirement: Display weight unit is per-device and applied at the boundary"
Cohesion: 0.13
Nodes (14): Purpose, Requirement: Current person's name is shown read-only with fallbacks, Requirement: Display weight unit is per-device and applied at the boundary, Requirement: Settings & Profile entry point on the dashboard, Requirements, Scenario: Changing the unit leaves stored values unchanged, Scenario: Default unit follows the device region, Scenario: Gear control opens Settings as its own screen (+6 more)

### Community 153 - "e2e-fixture-consumption.spec.ts"
Cohesion: 0.18
Nodes (12): collectFiles(), E2E_DIRECTORY, exportsValue(), ImportBinding, ImportDeclaration, parseImportBindings(), parseImports(), parseSpecifier() (+4 more)

### Community 154 - "sync/mod.rs"
Cohesion: 0.13
Nodes (3): OFFLINE_FAILURE_THRESHOLD, READ_REFRESH_TIMEOUT, SAVE_ATTEMPT_TIMEOUT

### Community 155 - "vitest"
Cohesion: 0.27
Nodes (8): vitest, checkConfigAgreement(), objectBlock(), parseTauriBuildTarget(), parseViteConfig(), TauriBuildTarget, ViteConfigFacts, stripComments()

### Community 156 - "csv"
Cohesion: 0.07
Nodes (14): detect_domain(), get_cip_brief(), _load_csv(), search(), search_all(), _search_csv(), detect_domain(), _load_csv() (+6 more)

### Community 157 - "test_design_system_mode.py"
Cohesion: 0.07
Nodes (14): _contrast_ratio(), _derive_dark_palette(), _filter_anti_patterns_for_mode(), _palette_is_dark(), _query_wants_dark(), _relative_luminance(), _resolve_color_mode(), _select_palette_for_mode() (+6 more)

### Community 159 - "ring"
Cohesion: 0.67
Nodes (3): ring, $type, $value

### Community 160 - "http.rs"
Cohesion: 0.14
Nodes (7): ACCOUNT_PATH, HEALTH_PATH, HOUSEHOLD_CHANGES_RPC, HttpService, RPC_PREFIX, transport_reply(), HTTP_UNAUTHORIZED

### Community 161 - "bindings.ts"
Cohesion: 0.04
Nodes (65): 1. Contract, 2. Backend (Rust), 3. Frontend logic, 4. Frontend UI, 5. Tests, 6. Review, Tasks — add-medication-tracking (P5), ADR 0001's follow-ups, mapped (task 1.1) (+57 more)

### Community 162 - "ADDED Requirements"
Cohesion: 0.14
Nodes (13): ADDED Requirements, Requirement: Current person's name is shown read-only with fallbacks, Requirement: Display weight unit is per-device and applied at the boundary, Requirement: Settings & Profile entry point on the dashboard, Scenario: Changing the unit leaves stored values unchanged, Scenario: Default unit follows the device region, Scenario: Gear control opens Settings as its own screen, Scenario: Resolved account name is shown read-only (+5 more)

### Community 163 - "settings.rs"
Cohesion: 0.31
Nodes (8): a_local_fault_carries_the_operation_identity_it_was_given(), a_local_fault_reports_the_unknown_cause_under_its_own_key_without_a_retry(), data_dir(), get_display_weight_unit(), local_fault_error(), off_thread(), set_display_weight_unit(), AppHandle

### Community 164 - "DeviceSnapshot"
Cohesion: 0.14
Nodes (11): CollectionStore, GatedSink, PersistenceWriter, RecordingSink, restored_book(), SnapshotSink, WriterState, CollectionStoreError (+3 more)

### Community 166 - "Tasks — add-household-sharing-hardening (P3)"
Cohesion: 0.25
Nodes (7): 1. Contract and native surface, 2. Backend (Rust), 3. Frontend logic, 4. Frontend UI, 5. Tests and manual verification, 6. Review, Tasks — add-household-sharing-hardening (P3)

### Community 168 - "Working agreement"
Cohesion: 0.40
Nodes (4): Reporting, Scope, The smallest complete change, Working agreement

### Community 174 - "SettingsScreen.tsx"
Cohesion: 0.05
Nodes (108): Q45 — The Settings refresh re-reads the list but not the active household, Q48 — The pending entry surface's only announced text is the primitive's fallback, Q61 — non-blocking copy/doc nits from the final 6.1 review, 0. Workspace scaffold, 7. Follow-up fixes (owner-approved 2026-10-21), 8. Review fixes (owner-approved 2026-10-22), Manual verification, react (+100 more)

### Community 177 - "Requirement: Household-scoped isolation and canonical storage at the boundary"
Cohesion: 0.17
Nodes (11): data-integrity Specification, Purpose, Requirement: Household-scoped isolation and canonical storage at the boundary, Requirement: Timestamps are absolute instants defaulting to now, Requirements, Scenario: A supplied instant is stored unchanged, Scenario: A timestamp defaults to now, Scenario: Canonical representation survives presentation changes (+3 more)

### Community 178 - "scripts"
Cohesion: 0.22
Nodes (9): scripts, build, dev, gen:bindings, lint, tauri, test, test:e2e (+1 more)

### Community 179 - "D0.9 — The bindings pipeline ships with an empty command set"
Cohesion: 0.18
Nodes (11): D0.9 — The bindings pipeline ships with an empty command set, No ADR number is taken by section 0, One builder, mounted and exported, Task 0.12's outcome — the fmt carve-out closes, and the integration harness lands, The empty command/event set it ships with, The export entry point is a cargo example, not a test, The module shape and the empty macro form (task 0.4), The single regeneration entry point (+3 more)

### Community 180 - "diagnostics/mod.rs"
Cohesion: 0.14
Nodes (7): lowercased_manifests(), neither_manifest_configures_a_third_party_analytics_transport(), DiagnosticOutcome, CollectionLoaded, ReachabilityProbed, SaveFailed, SaveSucceeded

### Community 181 - "dependencies"
Cohesion: 0.33
Nodes (6): dependencies, react, react-dom, @tanstack/react-query, @tanstack/react-router, @tauri-apps/api

### Community 182 - "Requirement: Household-scoped isolation and canonical storage at the boundary"
Cohesion: 0.18
Nodes (10): ADDED Requirements, data-integrity — delta for add-shared-household-foundation, Requirement: Household-scoped isolation and canonical storage at the boundary, Requirement: Timestamps are absolute instants defaulting to now, Scenario: A supplied instant is stored unchanged, Scenario: A timestamp defaults to now, Scenario: Canonical representation survives presentation changes, Scenario: No item is readable from another household (+2 more)

### Community 183 - "Design — add-shared-household-foundation (P0)"
Cohesion: 0.22
Nodes (8): Context, Design — add-shared-household-foundation (P0), Risks, Scaffold surface added by section 0, Scaffold traceability: `project-toolchain` (TOOL-1 … TOOL-8), State and boundaries, Traceability: user stories → scenarios, Verification

### Community 184 - "household-flow.spec.ts"
Cohesion: 0.31
Nodes (7): createFirstHousehold(), createHouseholdFromSettings(), householdNameField(), openEntry(), openSettingsFromDashboard(), RecorderWindow, screenTitle()

### Community 186 - "commands/household.rs"
Cohesion: 0.25
Nodes (14): a_refused_create_publishes_nothing(), a_refused_switch_publishes_nothing(), a_successful_create_publishes_exactly_one_created_for_the_new_household(), a_successful_switch_publishes_switched_for_the_household_now_active(), accepted_create(), account(), create_household(), created_household() (+6 more)

### Community 187 - "events.rs"
Cohesion: 0.11
Nodes (12): HouseholdChanged, .NAME, HouseholdChangeKind, Created, Refreshed, Removed, Switched, HouseholdChangePublisher (+4 more)

### Community 188 - "status.ts"
Cohesion: 0.10
Nodes (28): getDisplayWeightUnit(), getIdentity(), setDisplayWeightUnit(), DisplayWeightUnitFailure, DisplayWeightUnitKey, displayWeightUnitQueryOptions(), SetUnitOperation, UnitOutcome (+20 more)

### Community 189 - "path"
Cohesion: 0.10
Nodes (7): BINDINGS_PATH, builder(), export_bindings(), assert_send_sync(), building_the_ipc_builder_creates_removes_or_rewrites_no_source_file(), repository_src_entries(), the_ipc_builder_hands_out_a_mountable_handler_and_survives_being_moved()

### Community 190 - "offline-launch.spec.ts"
Cohesion: 0.25
Nodes (4): HOUSEHOLD, HOUSEHOLD_SUMMARY, LOADED_LOCAL_COPY, UNREACHABLE

### Community 191 - "ShadcnInstaller"
Cohesion: 0.05
Nodes (3): main(), ShadcnInstaller, TestShadcnInstaller

### Community 193 - "json"
Cohesion: 0.07
Nodes (6): test_sync_parses_bundled_starter_template(), main(), _run(), test_flags_hardcoded_hex_sharing_line_with_token(), test_token_only_line_reports_no_violation(), TestMetricMath

### Community 194 - "button"
Cohesion: 0.20
Nodes (10): fg, font-size, hover-bg, button, $type, $value, $type, $value (+2 more)

### Community 195 - "Tasks — add-read-path-performance (P6)"
Cohesion: 0.25
Nodes (7): 1. Contract and measurement design, 2. Backend (Rust), 3. Frontend logic, 4. Frontend UI, 5. Tests, 6. Review, Tasks — add-read-path-performance (P6)

### Community 197 - "add-shared-household-foundation"
Cohesion: 0.25
Nodes (7): add-shared-household-foundation, Dependencies, Dependency justifications (project rule 6), Impact, Settled decisions this change implements, What changes, Why

### Community 198 - "input"
Cohesion: 0.29
Nodes (8): padding-x, input, $type, $value, focus-ring, padding-x, $type, $value

### Community 201 - "$type"
Cohesion: 0.60
Nodes (5): $type, $value, border, border, border

### Community 202 - "radius"
Cohesion: 0.60
Nodes (5): radius, radius, radius, $type, $value

### Community 203 - "sync.rs"
Cohesion: 0.17
Nodes (11): assert_check_status_shape(), assert_get_status_shape(), check_service_reachability(), check_service_reachability_returns_and_publishes_the_trackers_new_status(), checked_status(), get_service_status(), get_service_status_answers_the_trackers_value_as_ok(), load_household() (+3 more)

### Community 204 - "lg"
Cohesion: 0.60
Nodes (5): lg, $type, $value, lg, lg

### Community 205 - "sm"
Cohesion: 0.60
Nodes (5): sm, sm, sm, $type, $value

### Community 206 - "padding-y"
Cohesion: 0.67
Nodes (4): padding-y, padding-y, $type, $value

### Community 207 - "16"
Cohesion: 0.67
Nodes (3): $type, $value, 16

### Community 210 - "D0.8 — Scaffold baseline and exact pins (section 0)"
Cohesion: 0.33
Nodes (6): Compatibility constraints (from the published metadata, not inferred), D0.8 — Scaffold baseline and exact pins (section 0), Exact pin set (task 0.2, TOOL-8), iOS and macOS support (TOOL-8, TOOL-1, TOOL-2), Materialised npm manifest (task 0.13), Pin provenance

### Community 211 - "1"
Cohesion: 0.67
Nodes (3): $type, $value, 1

### Community 214 - "3"
Cohesion: 0.67
Nodes (3): $type, $value, 3

### Community 215 - "8"
Cohesion: 0.67
Nodes (3): $type, $value, 8

### Community 216 - "destructive"
Cohesion: 0.67
Nodes (3): destructive, $type, $value

### Community 217 - "destructive-foreground"
Cohesion: 0.67
Nodes (3): destructive-foreground, $type, $value

### Community 218 - "Requirement: Continuous integration runs the same gate as the definition of done"
Cohesion: 0.40
Nodes (5): Requirement: Continuous integration runs the same gate as the definition of done, Scenario: CI covers no platform beyond iOS and macOS, Scenario: CI fails on generated drift, Scenario: CI neither signs nor distributes, Scenario: CI runs the gate [macos]

### Community 220 - "muted"
Cohesion: 0.67
Nodes (3): muted, $type, $value

### Community 221 - "primary-foreground"
Cohesion: 0.67
Nodes (3): primary-foreground, $type, $value

### Community 222 - "secondary-foreground"
Cohesion: 0.67
Nodes (3): secondary-foreground, $type, $value

### Community 226 - "D0.16 — Deployment targets are derived, not chosen (Q5)"
Cohesion: 0.67
Nodes (3): D0.16 — Deployment targets are derived, not chosen (Q5), The measured floor (task 0.7, `apple-native`), The shipped floor, raised by product decision (Q16, 2026-10-20)

### Community 228 - "Window"
Cohesion: 0.67
Nodes (3): D0.21 — Task 0.6: an empty permission set and a strict CSP (Q13), D0.24 — Task 0.17: the test layer has its own config, and an empty suite stays red, Window

## Ambiguous Edges - Review These
- `UI Styling Skill` → `Apache License 2.0`  [AMBIGUOUS]
  .opencode/skills/ui-styling/LICENSE.txt · relation: conceptually_related_to

## Knowledge Gaps
- **193 isolated node(s):** `Metric`, `NonMetric`, `Unknown`, `app`, `Malformed` (+188 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1645 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **53 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `types` connect `types` to `settings.rs`, `commands/identity.rs`, `sync.rs`, `compilerOptions`, `commands/household.rs`, `path`?**
  _High betweenness centrality (0.027) - this node is a cross-community bridge._
- **Are the 61 inferred relationships involving `block_on_ready()` (e.g. with `create_with()` and `a_refused_create_publishes_nothing()`) actually correct?**
  _`block_on_ready()` has 61 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Metric`, `NonMetric`, `Unknown` to the rest of the system?**
  _193 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `entry.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.14736842105263157 - nodes in this community are weakly interconnected._
- **Why does `compilerOptions` connect `compilerOptions` to `types`?**
  _High betweenness centrality (0.024) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `TailwindConfigGenerator` (e.g. with `TestGeneratedConfigIsValidJs` and `TestTailwindConfigGenerator`) actually correct?**
  _`TailwindConfigGenerator` has 2 INFERRED edges - model-reasoned connections that need verification._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `UI Styling Skill` and `Apache License 2.0`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._