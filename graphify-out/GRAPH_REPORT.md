# Graph Report - BunScale  (2026-10-16)

## Corpus Check
- 219 files · ~373,889 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 56 file(s) not represented in the graph (top: .csv 53, (none) 2, .jsonc 1)

## Summary
- 1754 nodes · 2851 edges · 124 communities (68 shown, 56 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 227 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `23e118d2`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- TestShadcnInstaller
- BM25 Search Core
- Design Token Values
- Design System Generator
- P1 IPC Contract Types
- Slide Search CLI
- Export & Reminders Decisions
- Weight & Archive Rules
- Health & Photos Design
- Agent Topology
- Design Skill References
- Data Contract Tests
- validate_data.py
- Reimplementation Domain Model
- Component Token Values
- Brand Token Sync Tests
- Icon Generation Pipeline
- UI Styling Concepts
- HTML Token Validator
- OpenSpec Workflow Skills
- ShadcnInstaller
- Logo Search Core
- Slide Deck Generation
- Design Reasoning Engine
- CIP HTML Renderer
- Tailwind Config Tests
- Sharing Runtime Behavior
- Background Image Fetching
- color
- Sharing Delivery Ordering
- Household Invitation Design
- Design Token Layers
- CIP Generation Pipeline
- Spacing Token Values
- Slides Copywriting & Template
- Brand Color Extraction
- Palette Luminance Tests
- Design Reasoning Contract
- Catalog Refresh Tests
- OpenSpec CLI Commands
- Brand Asset Validation
- CIP Search Core
- Web Stack Freshness Tests
- Brand Guidelines References
- Tailwind Breakpoints Tests
- Dark Palette Derivation
- Color Mode Resolution
- Domain Detection Tests
- Search Domain Tests
- Embed Tokens Script
- Token Validator Script
- button
- Multi-Domain Search
- Brand Context Injection
- Motion Token Values
- Tailwind Fonts & Spacing Tests
- Native Stack Freshness Tests
- Brand Token Sync Script
- Generate Tokens Script
- Component Specs & States
- Rust & Tauri Conventions
- BM25 Algorithm
- Weight Drop Alert Rules
- Radius Token Values
- Config Validation Tests
- json
- Anti-Pattern Mode Gating
- Relevance Threshold Gate
- Release & Definition of Done
- Tauri Security Rules
- Shadow Token Values
- Search Tokenizer Tests
- Style Identity Tests
- Frontend IPC & Errors
- Mobile Router & History
- Large Size Tokens
- BM25 Core Tests
- Relevance Metric Math
- Weight Validation Rules
- Frontend State & Routing
- Design Dials & Generation
- XL Size Tokens
- MD Size Tokens
- None Value Tokens
- Fixture Validation Tests
- Owner Attribution Sharing
- Dashboard & Timestamp Rules
- Design System Conventions
- Slide System & Tokens
- UI Styling Dependencies
- Quick Log Loop Rules
- Banner Design Skill
- Graphify Usage Workflow
- Member List Resolution
- Archive Order Resolution
- Dashboard Empty States
- Conflict Resolution Rules
- Card Component Spec
- Dialog Component Spec
- Route Guards
- Dashboard Never-Weighed State
- Multi-Rabbit Household
- Weight Entry Detail
- Household Onboarding
- Household Settings Entry
- Data Privacy Requirement
- Dual Viewport Test Harness
- CI Gate Requirement
- Settings Entry Point
- Read-Only Person Name
- P4 Frontend UI Tasks

## God Nodes (most connected - your core abstractions)
1. `TailwindConfigGenerator` - 58 edges
2. `DesignSystemGenerator` - 35 edges
3. `TestTailwindConfigGenerator` - 35 edges
4. `ShadcnInstaller` - 34 edges
5. `TestShadcnInstaller` - 26 edges
6. `Design Skill` - 24 edges
7. `UI Styling Skill` - 21 edges
8. `BunScale Reimplementation Requirements` - 18 edges
9. `Phase P4 — Handoff & preferences` - 17 edges
10. `read_rows()` - 16 edges

## Surprising Connections (you probably didn't know these)
- `P0 Change: add-shared-household-foundation` --conceptually_related_to--> `Architect Agent`  [INFERRED]
  docs/adr/0001-reimplementation-scope-decisions.md → .opencode/agents/architect.md
- `Reviewer Agent` --conceptually_related_to--> `Generated Files Are Read-Only`  [INFERRED]
  .opencode/agents/reviewer.md → AGENTS.md
- `oklch Color Space` --semantically_similar_to--> `CSS Variable Theming System`  [INFERRED] [semantically similar]
  .opencode/skills/ui-styling/references/canvas-design-system.md → .opencode/skills/ui-styling/SKILL.md
- `@theme Directive` --semantically_similar_to--> `CSS Variable Theming System`  [INFERRED] [semantically similar]
  .opencode/skills/ui-styling/references/tailwind-customization.md → .opencode/skills/ui-styling/SKILL.md
- `Apache License 2.0` --conceptually_related_to--> `UI Styling Skill`  [AMBIGUOUS]
  .opencode/skills/ui-styling/LICENSE.txt → .opencode/skills/ui-styling/SKILL.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Bounded load-once read path** — openspec_changes_add_read_path_performance_design_fixed_two_request_bound, openspec_changes_add_read_path_performance_design_freshness_window, openspec_changes_add_read_path_performance_design_load_once_per_area, openspec_changes_add_read_path_performance_design_scoped_invalidation [EXTRACTED 0.90]
- **Household sharing flows** — openspec_changes_add_household_sharing_hardening_design_one_invitation_reuse, openspec_changes_add_household_sharing_hardening_design_acceptance_adoption, openspec_changes_add_household_sharing_hardening_design_owner_gated_deletion_cascade, openspec_changes_add_household_sharing_hardening_design_local_first_member_list [EXTRACTED 0.90]
- **Near-simultaneous dose guard** — openspec_changes_add_medication_tracking_design_near_simultaneous_guard, openspec_changes_add_medication_tracking_design_entered_timestamp_anchor, openspec_changes_add_medication_tracking_design_fresh_read_guard [EXTRACTED 0.90]
- **Accessibility Pattern Set** — _opencode_skills_ui_styling_skill_radix_ui, _opencode_skills_ui_styling_references_shadcn_accessibility_wai_aria, _opencode_skills_ui_styling_references_shadcn_accessibility_focus_management, _opencode_skills_ui_styling_references_shadcn_accessibility_wcag_contrast, _opencode_skills_ui_styling_references_shadcn_accessibility [EXTRACTED 1.00]
- **ADR 0001 Reimplementation Scope Decisions** — docs_adr_0001_reimplementation_scope_decisions_d1_write_model, docs_adr_0001_reimplementation_scope_decisions_d2_sharing_model, docs_adr_0001_reimplementation_scope_decisions_d3_reminder_unification, docs_adr_0001_reimplementation_scope_decisions_d4_migration_fresh_start, docs_adr_0001_reimplementation_scope_decisions_d5_chart_baseline, docs_adr_0001_reimplementation_scope_decisions_d6_shared_service_deferred [EXTRACTED 1.00]
- **BunScale Agent Routing System** — agents_agent_routing, _opencode_agents_orchestrator_agent, _opencode_agents_spec_writer_agent, _opencode_agents_architect_agent, _opencode_agents_rust_backend_agent, _opencode_agents_apple_native_agent, _opencode_agents_frontend_logic_agent, _opencode_agents_frontend_ui_agent, _opencode_agents_tauri_security_agent, _opencode_agents_test_engineer_agent, _opencode_agents_reviewer_agent, _opencode_agents_release_ci_agent [EXTRACTED 1.00]
- **Archived immutability enforced at the write boundary** — openspec_changes_add_rabbit_weight_core_loop_design_d1_5, openspec_changes_add_rabbit_weight_core_loop_specs_data_integrity_spec_dm_4, openspec_changes_add_rabbit_weight_core_loop_specs_rabbit_profiles_spec_rab_8, openspec_changes_add_rabbit_weight_core_loop_specs_rabbit_profiles_spec_rab_9, openspec_changes_add_shared_household_foundation_specs_data_integrity_spec_nfr_10 [EXTRACTED 1.00]
- **Canonical grams stored, display unit applied only at the boundary** — openspec_changes_add_rabbit_weight_core_loop_design_d1_2, openspec_changes_add_rabbit_weight_core_loop_specs_data_integrity_spec_dm_1, openspec_changes_add_rabbit_weight_core_loop_specs_weight_tracking_spec_wt_3, openspec_changes_add_shared_household_foundation_specs_settings_profile_spec_set_4, openspec_changes_add_shared_household_foundation_specs_data_integrity_spec_nfr_10, openspec_changes_add_shared_household_foundation_design_d0_4 [EXTRACTED 1.00]
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

## Communities (124 total, 56 thin omitted)

### Community 1 - "BM25 Search Core"
Cohesion: 0.06
Nodes (31): BM25, _contains_phrase(), detect_domain(), _domain_keywords(), _exact_match_diagnostic(), _exact_row_identity(), _exact_stack_identifier(), _file_signature() (+23 more)

### Community 2 - "Design Token Values"
Cohesion: 0.05
Nodes (53): $type, $value, $type, $value, $type, $value, $type, $value (+45 more)

### Community 3 - "Design System Generator"
Cohesion: 0.06
Nodes (16): ansi_ljust(), _detect_page_type(), format_ascii_box(), format_markdown(), format_master_md(), format_page_override_md(), generate_design_system(), _generate_intelligent_overrides() (+8 more)

### Community 4 - "P1 IPC Contract Types"
Cohesion: 0.05
Nodes (30): DashboardRow shared type, P1 IPC contract (proposal, architect-owned), Rabbit shared type, WeightEntry shared type, WeightTrend shared type, P1 — MVP core loop, Q3 sex and neuter/spay status value sets, DASH-1 Dashboard lists non-archived rabbits with name, thumbnail and last-weighed date (+22 more)

### Community 5 - "Slide Search CLI"
Cohesion: 0.08
Nodes (17): format_context(), format_result(), main(), BM25, calculate_pattern_break(), detect_domain(), get_background_config(), get_color_for_emotion() (+9 more)

### Community 6 - "Export & Reminders Decisions"
Cohesion: 0.09
Nodes (24): P4 IPC contract (reminder, export, language commands and shared types), Reminder occurrence model (pending|fired|snoozed|satisfied|missed), P4 risk register (R1–R13), Phase P4 — Handoff & preferences, P4 settled questions (former Q1–Q6), EXP-1 — History exports as a formatted, vet-readable PDF for a selected range, EXP-2 — Same history exports as raw CSV with dedicated medication and dose files, EXP-3 — Exported files handed to the device's standard sharing options (+16 more)

### Community 7 - "Weight & Archive Rules"
Cohesion: 0.06
Nodes (27): DM-1 Weight is stored in grams as the single canonical unit, DM-4 An archived rabbit is read-only everywhere, RAB-7 An archived rabbit is restorable with its history intact, RAB-8 An archived rabbit's timeline is read-only, RAB-9 Archived rabbit data is retained by default, WT-3 Weight is entered and presented in the preferred unit without a unit picker, WT-5 A past weight entry is editable and deletable and the change is shared, Task 2.8 Enforce archived-rabbit immutability on every write path (+19 more)

### Community 8 - "Health & Photos Design"
Cohesion: 0.08
Nodes (24): Design — add-health-and-photo-history (P2), P2 IPC contract (proposed commands, events, shared types), Proposal — add-health-and-photo-history, P2 — Health & evidence phase, Questions — add-health-and-photo-history (P2), dashboard delta spec (DASH-4, DASH-7, HL-3), Dashboard severe-health flag (DASH-4 severe clause), History views (weight-only | health-only | combined) (+16 more)

### Community 9 - "Agent Topology"
Cohesion: 0.11
Nodes (25): Apple-Native Agent, Custom Swift Tauri Plugin, Architect Agent, IPC Contract (Commands, Events, Types), Frontend-Logic Agent, Typed IPC Wrapper (api.ts), Frontend-UI Agent, Design System Bootstrap (+17 more)

### Community 10 - "Design Skill References"
Cohesion: 0.11
Nodes (34): Banner Sizes & Art Direction Styles Reference, 22 Art Direction Styles, CIP Deliverable Guide, CIP Design Reference, CIP Mockup Prompt Engineering, CIP Base Prompt Structure, CIP Design Style Guide, CIP Color Psychology (+26 more)

### Community 11 - "Data Contract Tests"
Cohesion: 0.07
Nodes (6): main(), apply_decision_rules(), read_rows(), read_rows(), TestTextLayoutDataContracts, TestTextLayoutRetrieval

### Community 12 - "validate_data.py"
Cohesion: 0.06
Nodes (43): read_rows(), TestAccessibilityGuidance, TestChartsTypographyAndIcons, TestCurrentReactGuidance, TestSemanticColors, TestGeneratedCatalogContract, _catalog_date(), _check_app_interface_contract() (+35 more)

### Community 13 - "Reimplementation Domain Model"
Cohesion: 0.13
Nodes (27): P0 Change: add-shared-household-foundation, Dashboard Capability, BunScale Reimplementation Requirements, BunScale Domain Model, Export Capability, Health Logging Capability, Household Sharing Capability, Improvement Candidates (+19 more)

### Community 14 - "Component Token Values"
Cohesion: 0.06
Nodes (34): $type, $value, $type, $value, $type, $value, $type, $value (+26 more)

### Community 15 - "Brand Token Sync Tests"
Cohesion: 0.20
Nodes (4): test_sync_parses_bundled_starter_template(), _run(), test_flags_hardcoded_hex_sharing_line_with_token(), test_token_only_line_reports_no_violation()

### Community 16 - "Icon Generation Pipeline"
Cohesion: 0.10
Nodes (13): apply_color(), apply_viewbox_size(), extract_svgs(), generate_batch(), generate_icon(), generate_sizes(), load_env(), main() (+5 more)

### Community 17 - "UI Styling Concepts"
Cohesion: 0.11
Nodes (24): Apache License 2.0, Canvas Design System, oklch Color Space, shadcn/ui Accessibility Patterns, Focus Management, WAI-ARIA Design Patterns, WCAG Color Contrast, shadcn/ui Component Reference (+16 more)

### Community 18 - "HTML Token Validator"
Cohesion: 0.12
Nodes (12): get_context(), is_allowed_exception(), is_allowed_rgba(), is_inside_block(), load_css_variables(), main(), print_result(), print_summary() (+4 more)

### Community 19 - "OpenSpec Workflow Skills"
Cohesion: 0.15
Nodes (20): OpenSpec Apply Change Skill, OpenSpec Store Selection, OpenSpec Archive Change Skill, OpenSpec Continue Change Skill, Explore Mode, OpenSpec Explore Skill, OpenSpec New Change Skill, OpenSpec Onboard Skill (+12 more)

### Community 21 - "Logo Search Core"
Cohesion: 0.12
Nodes (6): BM25, detect_domain(), _load_csv(), search(), search_all(), _search_csv()

### Community 22 - "Slide Deck Generation"
Cohesion: 0.15
Nodes (9): _e(), generate_chart_slide(), generate_cta_slide(), generate_metrics_slide(), generate_problem_slide(), generate_solution_slide(), generate_testimonial_slide(), generate_title_slide() (+1 more)

### Community 24 - "CIP HTML Renderer"
Cohesion: 0.23
Nodes (4): generate_html(), get_deliverable_info(), get_image_base64(), main()

### Community 26 - "Sharing Runtime Behavior"
Cohesion: 0.11
Nodes (16): Live updates and background delivery handoff (D-H6), Status derived from fresh outcomes (D-H8), NotificationAuthorization state, Set-up-again recovery from shared service (D-H9), SharingPolicy configuration values, Visible household-naming notification (D-H14), HS-11 Changes reach every member without manual refresh, NFR-6 Honest service status (+8 more)

### Community 27 - "Background Image Fetching"
Cohesion: 0.16
Nodes (9): generate_css_for_background(), get_background_image(), get_curated_images(), get_overlay_css(), get_pexels_search_url(), load_backgrounds_config(), load_brand_colors(), main() (+1 more)

### Community 28 - "color"
Cohesion: 0.04
Nodes (48): $type, $value, background, destructive, destructive-foreground, foreground, muted, muted-foreground (+40 more)

### Community 29 - "Sharing Delivery Ordering"
Cohesion: 0.11
Nodes (17): Offline member deletion window (D-H15), Owner-gated irreversible deletion cascade (D-H5), Archive order P0-P1-P2-P3-P5-P4-P6, HS-10 Only the owner can permanently delete a household, Active/finished derived from end date (D5.2), Derived affordance set for read-only states (D5.6), Enumerated units with no conversion (D5.7), Dose giver captured from recording member (D5.4, DM-5) (+9 more)

### Community 30 - "Household Invitation Design"
Cohesion: 0.12
Nodes (13): Invitation acceptance as household adoption (D-H3), Member cap enforced at membership level (D-H13), One invitation per household, resolved by reuse (D-H2), Client-generated request_id idempotency, Convergence as primary risk (R1), Settled decisions D1-D6, HS-5 Inviting is availability-checked and never duplicated, HS-6 Accepting an invitation adopts the household (+5 more)

### Community 31 - "Design Token Layers"
Cohesion: 0.17
Nodes (10): Table Component Spec, Component Tokens Layer, Primitive Color Scales, Primitive Tokens, 4px Spacing Scale, Typography Scale, Semantic Tokens, shadcn/ui Alignment (+2 more)

### Community 32 - "CIP Generation Pipeline"
Cohesion: 0.19
Nodes (7): build_cip_prompt(), check_logo_required(), generate_cip_set(), generate_with_nano_banana(), load_env(), load_logo_image(), main()

### Community 33 - "Spacing Token Values"
Cohesion: 0.12
Nodes (16): $type, $value, $type, $value, $type, $value, $type, $value (+8 more)

### Community 35 - "Slides Copywriting & Template"
Cohesion: 0.15
Nodes (15): AIDA Copywriting Formula, Copywriting Formulas Reference, FAB Copywriting Formula, PAS Copywriting Formula, Slides Create Subcommand, Chart.js Integration, HTML Slide Template Reference, Feature Grid Layout (+7 more)

### Community 36 - "Brand Color Extraction"
Cohesion: 0.20
Nodes (11): calculateCompliance(), colorDistance(), displayPalette(), extractHexColors(), findNearestBrandColor(), fs, generateImageMagickCommand(), hexToRgb() (+3 more)

### Community 37 - "Palette Luminance Tests"
Cohesion: 0.18
Nodes (4): _palette_is_dark(), _relative_luminance(), TestEndToEndCoherence, TestLuminance

### Community 38 - "Design Reasoning Contract"
Cohesion: 0.13
Nodes (7): _object_without_duplicates(), parse_decision_rules(), _validate_action(), read_rows(), TestLandingAndStackContract, TestReasoningContract, _check_reasoning_contract()

### Community 40 - "OpenSpec CLI Commands"
Cohesion: 0.33
Nodes (10): opsx-apply command, opsx-archive command, opsx-continue command, opsx-explore command, opsx-new command, opsx-onboard command, opsx-propose command, opsx-sync command (+2 more)

### Community 41 - "Brand Asset Validation"
Cohesion: 0.25
Nodes (13): checkManifest(), formatBytes(), formatOutput(), fs, main(), parseFilename(), path, RULES (+5 more)

### Community 42 - "CIP Search Core"
Cohesion: 0.20
Nodes (6): detect_domain(), get_cip_brief(), _load_csv(), search(), search_all(), _search_csv()

### Community 45 - "Brand Guidelines References"
Cohesion: 0.21
Nodes (13): Asset Approval Checklist, Asset Organization Guide, Brand Guidelines Template, Color Palette Management, Brand Consistency Checklist, Logo Usage Rules, Messaging Framework, Typography Specifications (+5 more)

### Community 48 - "Dark Palette Derivation"
Cohesion: 0.22
Nodes (4): _contrast_ratio(), _derive_dark_palette(), _select_palette_for_mode(), TestPaletteSelection

### Community 49 - "Color Mode Resolution"
Cohesion: 0.11
Nodes (5): _query_wants_dark(), _resolve_color_mode(), _style_is_dark_primary(), TestModeResolution, TestStyleTaxonomy

### Community 53 - "Embed Tokens Script"
Cohesion: 0.17
Nodes (8): args, fs, minimal, MINIMAL_TOKENS, path, projectRoot, tokensPath, wrapStyle

### Community 54 - "Token Validator Script"
Cohesion: 0.24
Nodes (11): extensions, formatReport(), fs, getFiles(), main(), parseArgs(), path, patterns (+3 more)

### Community 55 - "button"
Cohesion: 0.06
Nodes (45): $type, $value, $type, $value, bg, fg, font-size, hover-bg (+37 more)

### Community 57 - "Brand Context Injection"
Cohesion: 0.31
Nodes (10): extractColorsFromTable(), extractCoreAttributes(), extractHexColors(), extractImageStyle(), extractTypography(), extractVoice(), fs, generatePromptAddition() (+2 more)

### Community 58 - "Motion Token Values"
Cohesion: 0.18
Nodes (11): fast, normal, slow, $type, $value, $type, $value, primitive (+3 more)

### Community 62 - "Brand Token Sync Script"
Cohesion: 0.29
Nodes (8): adjustBrightness(), { execFileSync }, extractColorsFromMarkdown(), fs, generateColorScale(), main(), path, updateDesignTokens()

### Community 63 - "Generate Tokens Script"
Cohesion: 0.36
Nodes (9): flattenTokens(), fs, generateCSS(), generateTailwind(), main(), parseArgs(), path, resolveReference() (+1 more)

### Community 66 - "Component Specs & States"
Cohesion: 0.22
Nodes (9): UI Task Additional Checks, Button Component Spec, Input Component Spec, Button Tokens, State Accessibility Requirements, Focus Ring, Interactive States, Icon Usage Rules (+1 more)

### Community 67 - "Rust & Tauri Conventions"
Cohesion: 0.22
Nodes (9): Tauri Event Subscription Pattern, Rust Async Conventions, Rust Module Structure, Apple Plugin Structure, Tauri Command Rules, Tauri Event Rules, Native Automation Limits, IPC Mocking (+1 more)

### Community 70 - "Weight Drop Alert Rules"
Cohesion: 0.28
Nodes (6): Q1 which entry is the previous entry on a shared timestamp, Q2 the drop alert as a pure derivation over the current entry set, DASH-4 (weight part) Rabbits with a weight-drop alert are flagged on the dashboard, RAB-5 Every field of an active rabbit's profile is editable and shared, WT-7 A weight drop beyond the threshold raises an alert and flags the rabbit, Task 2.13 Implement the drop-alert derivation

### Community 72 - "Radius Token Values"
Cohesion: 0.29
Nodes (8): $type, $value, $type, $value, radius, default, full, default

### Community 76 - "json"
Cohesion: 0.13
Nodes (7): format_brief(), format_results(), main(), format_output(), generate_design_brief(), generate_deck(), main()

### Community 80 - "Release & Definition of Done"
Cohesion: 0.33
Nodes (6): Definition of Done, Gate Command Sequence, iOS Signing and TestFlight Delivery, macOS Distribution Decision, Release CI Pipeline, Updater Security Rules

### Community 81 - "Tauri Security Rules"
Cohesion: 0.40
Nodes (6): Rust Platform Gating, Apple Plugin Rules, Tauri Capabilities, Content Security Policy, Per-platform Security Constraints, Tauri Scopes

### Community 82 - "Shadow Token Values"
Cohesion: 0.47
Nodes (6): sm, shadow, sm, sm, $type, $value

### Community 84 - "Style Identity Tests"
Cohesion: 0.47
Nodes (3): split_values(), style_identities(), TestStyleIdentityContract

### Community 85 - "Frontend IPC & Errors"
Cohesion: 0.40
Nodes (5): Feature Folder Structure, Frontend IPC Rules, Rust Error Conventions, AppError, tauri-specta Typed Bindings

### Community 86 - "Mobile Router & History"
Cohesion: 0.40
Nodes (5): Mobile-first CSS Rules, Platform Handling, Adaptive Router Layout, Hash History Setup, Test Viewports

### Community 89 - "Large Size Tokens"
Cohesion: 0.60
Nodes (5): lg, $type, $value, lg, lg

### Community 92 - "Weight Validation Rules"
Cohesion: 0.40
Nodes (3): NFR-9 Primary and destructive actions are visible and confirmed, RAB-10 An archived rabbit and its data are permanently deletable on confirmation, WT-4 Non-positive values are rejected and implausible outliers are confirmed

### Community 93 - "Frontend State & Routing"
Cohesion: 0.50
Nodes (4): Lifecycle and Connectivity Handling, Frontend State Management Matrix, Router Data Loading, Typed Search Params

### Community 94 - "Design Dials & Generation"
Cohesion: 0.50
Nodes (3): Design Dials, AI Design System Generation, ui-ux-pro-max Query Contract

### Community 96 - "XL Size Tokens"
Cohesion: 0.67
Nodes (4): xl, xl, $type, $value

### Community 97 - "MD Size Tokens"
Cohesion: 0.67
Nodes (4): $type, $value, md, md

### Community 98 - "None Value Tokens"
Cohesion: 0.67
Nodes (4): $type, $value, none, none

### Community 100 - "Owner Attribution Sharing"
Cohesion: 0.50
Nodes (4): Owner attribution as hard selection criterion (D-H16), Owner-attributed storage allowance (D-H7), No open questions (P3), HS-12 Shared content counts against the owner's allowance

### Community 101 - "Dashboard & Timestamp Rules"
Cohesion: 0.50
Nodes (3): DASH-3 Rabbits are sorted by most recent weigh-in with never-weighed last, WT-1 A weight entry carries a value, a timestamp and an optional note, DM-6 Timestamps are absolute instants defaulting to now

### Community 104 - "UI Styling Dependencies"
Cohesion: 0.67
Nodes (3): UI Styling Skill Python Dependencies, pytest, UI Styling Skill Test Requirements

## Ambiguous Edges - Review These
- `UI Styling Skill` → `Apache License 2.0`  [AMBIGUOUS]
  .opencode/skills/ui-styling/LICENSE.txt · relation: conceptually_related_to

## Knowledge Gaps
- **156 isolated node(s):** `Banner Design - Multi-Format Creative Banner System`, `Main Spec Format`, `Capability Retirement`, `22 Art Direction Styles`, `Core Logo Types` (+151 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 701 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **56 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ShadcnInstaller` connect `ShadcnInstaller` to `.get_installed_components`, `.check_shadcn_config`, `TestShadcnInstaller`, `.__init__`, `.add_components`, `Brand Token Sync Tests`, `.add_all_components`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `TailwindConfigGenerator` (e.g. with `TestGeneratedConfigIsValidJs` and `TestTailwindConfigGenerator`) actually correct?**
  _`TailwindConfigGenerator` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Banner Design - Multi-Format Creative Banner System`, `Main Spec Format`, `Capability Retirement` to the rest of the system?**
  _156 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `TestShadcnInstaller` be split into smaller, more focused modules?**
  _Cohesion score 0.11578947368421053 - nodes in this community are weakly interconnected._
- **Why does `TailwindConfigGenerator` connect `Config Validation Tests` to `Tailwind Config Generator`, `.add_colors`, `.recommend_plugins`, `Plugin Config Tests`, `Tailwind Breakpoints Tests`, `Tailwind Config Tests`, `Tailwind Fonts & Spacing Tests`, `Tailwind Base Config`?**
  _High betweenness centrality (0.031) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `DesignSystemGenerator` (e.g. with `TestReasoningMatch` and `TestReasoningContract`) actually correct?**
  _`DesignSystemGenerator` has 3 INFERRED edges - model-reasoned connections that need verification._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `UI Styling Skill` and `Apache License 2.0`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._