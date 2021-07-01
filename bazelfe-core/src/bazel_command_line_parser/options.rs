use super::*;
use lazy_static::lazy_static;
lazy_static! {
    pub static ref ALL_ACTION_OPTIONS: Vec<BazelOption> = {
        let mut vec = Vec::new();
        vec.push(BazelOption::BooleanOption(
            String::from("action_cache"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("allow_analysis_failures"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("android_databinding_use_androidx"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("android_resource_shrinking"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("announce"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("announce_rc"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("apple_enable_auto_dsym_dbg"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("apple_generate_dsym"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("async"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("attempt_to_print_relative_paths"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("bep_publish_used_heap_size_post_build"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("bes_best_effort"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("bes_lifecycle_events"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("build"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("build_event_binary_file_path_conversion"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_event_json_file_path_conversion"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_event_publish_all_actions"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_event_text_file_path_conversion"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_manual_tests"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_python_zip"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_runfile_links"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_runfile_manifests"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_test_dwp"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("build_tests_only"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("cache_test_results"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("canonicalize_policy"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("check_licenses"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("check_tests_up_to_date"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("check_up_to_date"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("check_visibility"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("collapse_duplicate_defines"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("collect_code_coverage"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("compile_one_dependency"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("configure"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("desugar_for_android"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("device_debug_entitlements"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("discard_analysis_cache"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("distinct_host_configuration"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("enable_apple_binary_native_protos"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("enable_fdo_profile_absolute_path"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("enable_platform_specific_config"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("enable_runfiles"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("enforce_constraints"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("expand_test_suites"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_allow_android_library_deps_without_srcs"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_allow_tags_propagation"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_android_compress_java_resources"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_android_resource_shrinking"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_android_rewrite_dexes_with_rex"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_announce_profile_path"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_build_event_expand_filesets"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_build_event_fully_resolve_fileset_symlinks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_cancel_concurrent_tests"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_cc_shared_library"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_check_desugar_deps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_convenience_symlinks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_convenience_symlinks_bep_event"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_delay_virtual_input_materialization"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_disable_external_package"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_docker_privileged"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_docker_use_customized_images"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_docker_verbose"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_enable_android_migration_apis"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_enable_docker_sandbox"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_enable_objc_cc_deps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_extra_action_top_level_only"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_fetch_all_coverage_outputs"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_filter_library_jar_with_program_jar"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_forward_instrumented_files_info_by_default"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_generate_json_trace_profile"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_generate_llvm_lcov"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_google_legacy_api"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_graphless_query"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_guard_against_concurrent_changes"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_inmemory_dotd_files"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_inmemory_jdeps_files"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_inprocess_symlink_creation"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_interleave_loading_and_analysis"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_java_proto_add_allowed_public_imports"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_local_memory_estimate"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_materialize_param_files_directly"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_multi_threaded_digest"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_ninja_actions"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_objc_enable_module_maps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_objc_include_scanning"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_omitfp"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_persistent_test_runner"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_platforms_api"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_prefer_mutual_xcode"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_profile_cpu_usage"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_profile_include_primary_output"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_profile_include_target_label"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_proto_descriptor_sets_include_source_info"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_proto_extra_actions"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_remotable_source_manifests"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_remote_execution_keepalive"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_repo_remote_exec"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_repository_cache_hardlinks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_run_validations"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_sandboxfs_map_symlink_targets"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_save_feature_state"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_sibling_repository_layout"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_split_coverage_postprocessing"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_split_xml_generation"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_starlark_cc_import"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_starlark_config_transitions"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_stream_log_file_uploads"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_strict_fileset_output"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_use_llvm_covmap"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_use_sandboxfs"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_use_windows_sandbox"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_windows_watchfs"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("experimental_worker_multiplex"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("explicit_java_test_deps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("expunge"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("fat_apk_hwasan"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("force_pic"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("gnu_format"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("google_default_credentials"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("ignore_unsupported_sandboxing"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("implicit_deps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("include_artifacts"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("include_aspects"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("include_commandline"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("include_param_files"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_always_check_depset_elements"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_avoid_conflict_dlls"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_default_to_explicit_init_py"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_depset_for_libraries_to_link_getter"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disable_depset_items"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disable_expand_if_all_available_in_flag_set"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disable_native_android_rules"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disable_target_provider_fields"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disable_third_party_license_checking"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disallow_empty_glob"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disallow_legacy_javainfo"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disallow_legacy_py_provider"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_disallow_struct_provider_syntax"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_display_source_file_location"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_do_not_split_linking_cmdline"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_dont_enable_host_nonhost_crosstool_features"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_enable_android_toolchain_resolution"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_exclusive_test_sandboxed"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_force_strict_header_check_from_starlark"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_java_common_parameters"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_linkopts_to_linklibs"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_make_thinlto_command_lines_standalone"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_merge_genfiles_directory"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_new_actions_api"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_no_attr_license"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_no_implicit_file_export"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_no_rule_outputs_param"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_objc_compile_info_migration"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_objc_provider_remove_compile_info"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_prefer_unordered_output"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_remote_results_ignore_disk"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_remote_symlinks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_remove_cpu_and_compiler_attributes_from_cc_toolchain"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_remove_legacy_whole_archive"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_require_ctx_in_configure_features"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_require_linker_input_cc_api"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_restrict_string_escapes"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_run_shell_command_string"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_strict_action_env"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_struct_has_no_methods"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_use_cc_configure_from_rules_cc"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_use_platforms_repo_for_constraints"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_use_python_toolchains"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_validate_top_level_header_inclusions"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_visibility_private_attributes_at_definition"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incremental"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incremental_dexing"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("infer_universe_scope"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("instrument_test_targets"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("interface_shared_objects"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("ios_memleaks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("java_deps"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("java_header_compilation"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("json_trace_compression"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("keep_going"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("keep_state_after_build"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("legacy_external_runfiles"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("legacy_important_outputs"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("legacy_whole_archive"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("line_terminator_null"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("materialize_param_files"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("nodep_deps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("objc_enable_binary_stripping"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("objc_generate_linkmap"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("objc_use_dotd_pruning"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("packages"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("parse_headers_verifies_modules"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("print_relative_test_log_paths"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("process_headers_in_dependencies"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("progress_in_terminal_title"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("relative_locations"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("remote_accept_cached"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("remote_allow_symlink_upload"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("remote_local_fallback"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("remote_upload_local_results"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("remote_verify_downloads"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("rule_classes"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("rules"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("runs_per_test_detects_flakes"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("sandbox_debug"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("sandbox_default_allow_network"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("sandbox_fake_hostname"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("sandbox_fake_username"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("save_temps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("share_native_deps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("show_loading_progress"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("show_make_env"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("show_progress"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("show_task_finish"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("show_timestamps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("show_warnings"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("skyframe_state"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("slim_profile"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("split_apks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("stamp"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("strict_filesets"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("strict_system_includes"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("strict_test_suite"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("subcommands"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("test_keep_going"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("test_runner_fail_fast"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("test_verbose_timeout_warnings"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("tool_deps"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("toolchain_resolution_debug"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("track_incremental_state"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("translations"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("trim_test_configuration"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("use_ijars"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("use_singlejar_apkbuilder"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("verbose_explanations"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("verbose_failures"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("verbose_test_summary"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("watchfs"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("worker_quit_after_build"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("worker_sandboxing"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("worker_verbose"),
            false,
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("action_env"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("action_graph"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("adb"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("adb_arg"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("all_incompatible_changes"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("analysis_testing_deps_limit"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_compiler"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_cpu"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_crosstool_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_dynamic_mode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_grte_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_manifest_merger"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_manifest_merger_order"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("android_sdk"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("apk_signing_method"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("apple_bitcode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("apple_compiler"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("apple_crosstool_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("apple_grte_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("apple_sdk"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("aspect_deps"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("aspects"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("auto_cpu_environment_group"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("auto_output_filter"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_backend"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_keywords"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_outerr_buffer_size"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_outerr_chunk_size"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_proxy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_results_url"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bes_timeout"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("build_event_binary_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("build_event_json_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("build_event_max_named_set_of_file_entries"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("build_event_text_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("build_metadata"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("build_tag_filters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("catalyst_cpus"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cc_output_directory_tag"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cc_proto_library_header_suffixes"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cc_proto_library_source_suffixes"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("check_constraint"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("color"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("combined_report"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("compilation_mode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("compiler"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("config"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("conlyopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("copt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("coverage_report_generator"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("coverage_support"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cpu"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("crosstool_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cs_fdo_absolute_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cs_fdo_instrument"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cs_fdo_profile"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("curses"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("custom_malloc"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("cxxopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("debug_app"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("define"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("deleted_packages"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("device"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("disk_cache"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("distdir"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("dump"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("dynamic_mode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("embed_label"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("execution_log_binary_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("execution_log_json_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_action_listener"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_add_exec_constraints_to_targets"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_build_event_upload_strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_docker_image"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_downloader_config"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_execution_log_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_extra_action_filter"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_import_deps_checking"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_java_classpath"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_local_execution_delay"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_multi_cpu"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_objc_fastbuild_options"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_oom_more_eagerly_threshold"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_persistent_javac"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_profile_additional_tasks"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_remote_downloader"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_remote_grpc_log"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_repository_hash_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_repository_resolved_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_resolved_file_instead_of_workspace"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_sandbox_async_tree_delete_idle_threads"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_sandboxfs_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_scale_timeouts"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_spawn_scheduler"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_strict_java_deps"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_ui_max_stdouterr_bytes"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_ui_mode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_verify_repository_rules"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_windows_sandbox_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_worker_max_multiplex_instances"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("experimental_workspace_rules_log_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("explain"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("expunge_async"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("extra_execution_platforms"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("extra_toolchains"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("fat_apk_cpu"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("fdo_instrument"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("fdo_optimize"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("fdo_prefetch_hints"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("fdo_profile"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("features"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("fission"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("flag_alias"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("flaky_test_attempts"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("for_command"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("genrule_strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("google_auth_scopes"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("google_credentials"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("grpc_keepalive_time"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("grpc_keepalive_timeout"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("grte_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("help_verbosity"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("high_priority_workers"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_action_env"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_compilation_mode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_compiler"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_conlyopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_copt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_cpu"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_crosstool_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_cxxopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_force_python"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_grte_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_java_launcher"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_java_toolchain"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_javabase"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_javacopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_linkopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_platform"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_swiftcopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("http_timeout_scaling"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("iff_heap_size_greater_than"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("incremental_install_verbosity"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("instrumentation_filter"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("invocation_policy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_cpu"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_minimum_os"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_multi_cpus"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_sdk_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_signing_cert_name"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_simulator_device"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ios_simulator_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("java_debug"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("java_launcher"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("java_toolchain"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("javabase"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("javacopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("jobs"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("jvmopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("legacy_main_dex_list_generator"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("linkopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("loading_phase_threads"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("local_cpu_resources"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("local_ram_resources"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("local_termination_grace_seconds"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("local_test_jobs"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("logging"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("long"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ltobackendopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ltoindexopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("macos_cpus"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("macos_minimum_os"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("macos_sdk_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("max_computation_steps"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("max_config_changes_to_show"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("max_test_output_bytes"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("memory_profile_stable_heap_parameters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("message_translations"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("minimum_os_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("mode"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("modify_execution_info"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("nested_set_depth_limit"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("noorder_results"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("null"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("objccopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("only"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("order_output"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("order_results"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("output"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("output_filter"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("output_groups"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("override_repository"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("package_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("per_file_copt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("per_file_ltobackendopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("persistent_android_resource_processor"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("platform_mappings"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("platform_suffix"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("platforms"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("plugin"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("print_action_mnemonics"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("profile"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("progress_report_interval"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("proguard_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("project_id"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("propeller_optimize"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("proto_compiler"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("proto_toolchain_for_cc"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("proto_toolchain_for_java"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("proto_toolchain_for_javalite"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("protocopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("python_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("python_top"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("python_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("query_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_cache"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_cache_header"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_default_exec_properties"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_default_platform_properties"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_download_minimal"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_download_outputs"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_download_symlink_template"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_download_toplevel"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_downloader_header"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_exec_header"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_execution_priority"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_executor"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_header"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_instance_name"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_local_fallback_strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_max_connections"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_proxy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_result_cache_priority"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_retries"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("remote_timeout"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("repo_env"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("repository_cache"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("run_under"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("runs_per_test"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("sandbox_add_mount_pair"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("sandbox_base"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("sandbox_block_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("sandbox_tmpfs_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("sandbox_writable_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("script_path"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("shell_executable"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("short"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("show_config_fragments"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("show_progress_rate_limit"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("show_result"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("skyframe"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("skylark_memory"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("spawn_strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("starlark_cpu_profile"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("start"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("start_app"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("strategy_regexp"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("strict_proto_deps"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("strip"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("stripopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("swiftcopt"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("symlink_prefix"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("target_environment"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("target_pattern_file"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("target_platform_fallback"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_arg"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_env"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_filter"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_lang_filters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_output"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_result_expiration"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_sharding_strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_size_filters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_strategy"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_summary"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_tag_filters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_timeout"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_timeout_filters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("test_tmpdir"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tls_certificate"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tls_client_certificate"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tls_client_key"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tool_tag"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("transitions"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tvos_cpus"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tvos_minimum_os"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tvos_sdk_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tvos_simulator_device"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("tvos_simulator_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ui_actions_shown"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("ui_event_filters"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("universe_scope"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("watchos_cpus"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("watchos_minimum_os"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("watchos_sdk_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("watchos_simulator_device"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("watchos_simulator_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("worker_extra_flag"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("worker_max_instances"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("workspace_status_command"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("xbinary_fdo"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("xcode_version"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("xcode_version_config"),
            String::default(),
        ));
        vec
    };
}

lazy_static! {
    pub static ref STARTUP_OPTIONS: Vec<BazelOption> = {
        let mut vec = Vec::new();
        vec.push(BazelOption::BooleanOption(
            String::from("autodetect_server_javabase"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("batch"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("batch_cpu_scheduling"),
            false,
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("bazelrc"),
            String::default(),
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("block_for_lock"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("client_debug"),
            false,
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("connect_timeout_secs"),
            String::default(),
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("expand_configs_in_place"),
            false,
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("failure_detail_out"),
            String::default(),
        ));
        vec.push(BazelOption::BooleanOption(String::from("home_rc"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("idle_server_tasks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("ignore_all_rc_files"),
            false,
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("io_nice_level"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("local_startup_timeout_secs"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("macos_qos_class"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("max_idle_secs"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("output_base"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("output_user_root"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("server_jvm_out"),
            String::default(),
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("shutdown_on_low_sys_mem"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("system_rc"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("unlimit_coredumps"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(String::from("watchfs"), false));
        vec.push(BazelOption::BooleanOption(
            String::from("windows_enable_symlinks"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("workspace_rc"),
            false,
        ));
        vec.push(BazelOption::BooleanOption(
            String::from("incompatible_enable_execution_transition"),
            false,
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_jvm_args"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_jvm_debug"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("host_jvm_profile"),
            String::default(),
        ));
        vec.push(BazelOption::OptionWithArg(
            String::from("server_javabase"),
            String::default(),
        ));
        vec
    };
}

lazy_static! {
    pub static ref ACTION_TO_OPTIONS: std::collections::HashMap<BuiltInAction, Vec<usize>> = {
        let mut map = std::collections::HashMap::new();
        map.insert(
            BuiltInAction::AnalyzeProfile,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 81, 82, 87,
                89, 90, 91, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133, 134, 135,
                137, 139, 144, 145, 148, 149, 150, 151, 153, 160, 161, 162, 164, 165, 169, 178,
                180, 182, 194, 212, 213, 214, 217, 229, 237, 245, 265, 266, 267, 268, 269, 270,
                271, 272, 273, 274, 275, 276, 283, 287, 297, 305, 306, 313, 315, 323, 325, 328,
                330, 333, 336, 337, 338, 341, 357, 358, 359, 360, 381, 407, 414, 417, 422, 432,
                442, 445, 477, 489, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map.insert(
            BuiltInAction::Aquery,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 120, 121, 122, 123, 124, 125, 126,
                127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142,
                143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 155, 156, 157, 158, 159,
                160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 171, 172, 173, 174, 175, 176,
                177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 191, 193, 194,
                195, 196, 197, 198, 199, 200, 203, 204, 205, 206, 207, 208, 209, 210, 212, 213,
                214, 216, 217, 219, 220, 221, 223, 224, 225, 227, 228, 229, 230, 231, 232, 233,
                234, 235, 237, 238, 239, 240, 241, 245, 246, 247, 248, 249, 250, 251, 252, 253,
                254, 255, 256, 257, 258, 259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269,
                270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285,
                286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 301, 302,
                304, 305, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320,
                321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336,
                337, 338, 339, 340, 341, 342, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353,
                354, 356, 357, 358, 359, 360, 361, 363, 364, 365, 366, 367, 368, 369, 370, 371,
                372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 384, 386, 387, 388, 389, 390,
                391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403, 404, 405, 406,
                407, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 421, 422, 425, 429,
                430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440, 442, 443, 444, 445, 446,
                447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458, 459, 460, 461, 462, 463,
                464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476, 477, 478, 479,
                480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497, 498, 499, 500, 501, 502,
                503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515, 516, 517, 518,
                519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530, 531, 532, 533, 534, 535,
                536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Build,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
                185, 187, 188, 189, 191, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205, 206,
                207, 208, 209, 210, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225, 228, 229,
                230, 231, 232, 233, 234, 235, 237, 238, 239, 240, 241, 245, 246, 247, 248, 249,
                250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263, 264, 265, 266,
                267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282,
                283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298,
                299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317,
                318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333,
                334, 335, 336, 337, 338, 339, 340, 341, 342, 344, 345, 346, 347, 348, 349, 350,
                351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365, 366, 367, 368,
                369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 384, 386, 387,
                388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403,
                404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 421,
                422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440, 442, 443, 444,
                445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458, 459, 460, 461,
                462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476, 477,
                478, 479, 480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497, 498, 499, 500,
                501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515, 516,
                517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530, 531, 532, 534,
                535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::CanonicalizeFlags,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 25, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 72, 73,
                81, 82, 87, 89, 90, 91, 95, 96, 97, 101, 105, 106, 111, 118, 120, 122, 125, 128,
                129, 132, 133, 134, 135, 137, 138, 139, 144, 145, 148, 149, 150, 151, 153, 154,
                155, 156, 160, 161, 162, 164, 165, 169, 172, 178, 179, 180, 182, 184, 186, 194,
                195, 196, 197, 198, 199, 200, 210, 212, 213, 214, 215, 217, 222, 227, 229, 237,
                245, 261, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 283, 287,
                297, 302, 304, 305, 313, 315, 323, 325, 326, 327, 328, 329, 330, 333, 336, 337,
                338, 341, 355, 357, 358, 359, 360, 381, 385, 402, 407, 414, 417, 422, 423, 424,
                427, 428, 429, 432, 433, 442, 445, 455, 456, 457, 458, 459, 460, 461, 462, 463,
                464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 477, 489, 494, 521,
                522, 523, 524, 531, 532, 533,
            ],
        );
        map.insert(
            BuiltInAction::Clean,
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46,
                47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67,
                68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
                90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107,
                108, 109, 110, 111, 112, 113, 114, 115, 116, 118, 119, 125, 126, 127, 128, 129,
                130, 131, 132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146,
                147, 148, 149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163,
                164, 165, 166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181,
                182, 183, 185, 187, 188, 189, 191, 193, 194, 196, 197, 198, 199, 200, 203, 204,
                205, 206, 207, 208, 209, 210, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225,
                228, 229, 230, 231, 232, 233, 234, 235, 237, 238, 239, 240, 241, 245, 246, 247,
                248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263, 264,
                265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280,
                281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296,
                297, 298, 299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314, 315,
                316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331,
                332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 343, 344, 345, 346, 347,
                348, 349, 350, 351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365,
                366, 367, 368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381,
                384, 386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400,
                401, 402, 403, 404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417,
                418, 419, 421, 422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440,
                442, 443, 444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458,
                459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474,
                475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497,
                498, 499, 500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513,
                514, 515, 516, 517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530,
                531, 532, 534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Coverage,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
                185, 187, 188, 189, 191, 192, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205,
                206, 207, 208, 209, 210, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225, 226,
                228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 245, 246,
                247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263,
                264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279,
                280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295,
                296, 297, 298, 299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314,
                315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330,
                331, 332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 344, 345, 346, 347,
                348, 349, 350, 351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365,
                366, 367, 368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381,
                384, 386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400,
                401, 402, 403, 404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417,
                418, 419, 421, 422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440,
                442, 443, 444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458,
                459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474,
                475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497,
                498, 499, 500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513,
                514, 515, 516, 517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530,
                531, 532, 534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Cquery,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 120, 122, 125, 126, 127, 128, 129,
                130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145,
                146, 147, 148, 149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162,
                163, 164, 165, 166, 167, 168, 169, 171, 172, 173, 174, 175, 176, 177, 178, 179,
                180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 191, 193, 194, 195, 196, 197,
                198, 199, 200, 203, 204, 205, 206, 207, 208, 209, 210, 212, 213, 214, 217, 219,
                220, 221, 223, 224, 225, 227, 228, 229, 230, 231, 232, 233, 234, 235, 237, 238,
                239, 240, 241, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257,
                258, 259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273,
                274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289,
                290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 301, 302, 304, 305, 307, 308,
                309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324,
                325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336, 337, 338, 339, 340,
                341, 342, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353, 354, 356, 357, 358,
                359, 360, 361, 363, 364, 365, 366, 367, 368, 369, 370, 371, 372, 373, 374, 375,
                376, 377, 378, 379, 380, 381, 384, 386, 387, 388, 389, 390, 391, 392, 393, 394,
                395, 396, 397, 398, 399, 400, 401, 402, 403, 404, 405, 406, 407, 409, 410, 411,
                412, 413, 414, 415, 416, 417, 418, 419, 421, 422, 425, 429, 430, 431, 432, 433,
                434, 435, 436, 437, 438, 439, 440, 442, 443, 444, 445, 446, 447, 448, 449, 450,
                451, 452, 453, 454, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467,
                468, 469, 470, 471, 472, 473, 474, 475, 476, 477, 478, 479, 480, 481, 482, 483,
                484, 486, 488, 489, 490, 493, 494, 497, 498, 499, 500, 501, 502, 503, 504, 505,
                506, 507, 508, 509, 510, 511, 512, 513, 514, 515, 516, 517, 518, 519, 520, 521,
                522, 523, 524, 525, 526, 527, 528, 529, 530, 531, 532, 533, 534, 535, 536, 537,
                538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Dump,
            vec![
                0, 5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 81, 82,
                87, 89, 90, 91, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133, 134, 135,
                137, 139, 144, 145, 148, 149, 150, 151, 153, 160, 161, 162, 164, 165, 169, 178,
                180, 182, 190, 194, 201, 202, 212, 213, 214, 217, 229, 237, 242, 245, 265, 266,
                267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 283, 287, 297, 305, 313, 315,
                323, 325, 328, 330, 333, 336, 337, 338, 341, 357, 358, 359, 360, 381, 407, 414,
                417, 422, 432, 442, 445, 477, 489, 491, 492, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map.insert(
            BuiltInAction::Fetch,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 73, 81, 82,
                87, 89, 90, 91, 95, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133, 134,
                135, 137, 139, 144, 145, 148, 149, 150, 151, 153, 155, 156, 160, 161, 162, 164,
                165, 169, 178, 179, 180, 182, 194, 196, 197, 198, 199, 200, 210, 212, 213, 214,
                217, 229, 237, 245, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276,
                283, 287, 297, 302, 304, 305, 313, 315, 323, 325, 326, 327, 328, 329, 330, 333,
                336, 337, 338, 341, 357, 358, 359, 360, 381, 402, 407, 414, 417, 422, 432, 433,
                442, 445, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469,
                470, 471, 472, 473, 474, 475, 477, 489, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map.insert(
            BuiltInAction::Help,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 81, 82, 87,
                89, 90, 91, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133, 134, 135,
                137, 139, 144, 145, 148, 149, 150, 151, 153, 160, 161, 162, 164, 165, 169, 178,
                180, 182, 194, 212, 213, 214, 217, 229, 237, 245, 265, 266, 267, 268, 269, 270,
                271, 272, 273, 274, 275, 276, 283, 287, 297, 305, 313, 315, 323, 325, 328, 330,
                333, 336, 337, 338, 341, 357, 358, 359, 360, 362, 381, 407, 408, 414, 417, 422,
                432, 442, 445, 477, 487, 489, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map.insert(
            BuiltInAction::Info,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
                185, 187, 188, 189, 191, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205, 206,
                207, 208, 209, 210, 211, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225, 228,
                229, 230, 231, 232, 233, 234, 235, 237, 238, 239, 240, 241, 245, 246, 247, 248,
                249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263, 264, 265,
                266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281,
                282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297,
                298, 299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316,
                317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332,
                333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 344, 345, 346, 347, 348, 349,
                350, 351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365, 366, 367,
                368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 384, 386,
                387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402,
                403, 404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419,
                421, 422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440, 442, 443,
                444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458, 459, 460,
                461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476,
                477, 478, 479, 480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497, 498, 499,
                500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515,
                516, 517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530, 531, 532,
                534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::License,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 81, 82, 87,
                89, 90, 91, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133, 134, 135,
                137, 139, 144, 145, 148, 149, 150, 151, 153, 160, 161, 162, 164, 165, 169, 178,
                180, 182, 194, 212, 213, 214, 217, 229, 237, 245, 265, 266, 267, 268, 269, 270,
                271, 272, 273, 274, 275, 276, 283, 287, 297, 305, 313, 315, 323, 325, 328, 330,
                333, 336, 337, 338, 341, 357, 358, 359, 360, 381, 407, 414, 417, 422, 432, 442,
                445, 477, 489, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map.insert(
            BuiltInAction::MobileInstall,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 170, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182,
                183, 185, 187, 188, 189, 191, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205,
                206, 207, 208, 209, 210, 212, 213, 214, 217, 218, 219, 220, 221, 223, 224, 225,
                228, 229, 230, 231, 232, 233, 234, 235, 237, 238, 239, 240, 241, 243, 244, 245,
                246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262,
                263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278,
                279, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294,
                295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305, 307, 308, 309, 310, 311,
                312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327,
                328, 329, 330, 331, 332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 344,
                345, 346, 347, 348, 349, 350, 351, 352, 353, 354, 356, 357, 358, 359, 360, 361,
                363, 364, 365, 366, 367, 368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378,
                379, 380, 381, 383, 384, 386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396,
                397, 398, 399, 400, 401, 402, 403, 404, 405, 406, 407, 409, 410, 411, 412, 413,
                414, 415, 416, 417, 418, 419, 420, 421, 422, 425, 430, 431, 432, 433, 434, 435,
                436, 437, 438, 439, 440, 442, 443, 444, 445, 446, 447, 448, 449, 450, 451, 452,
                453, 454, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469,
                470, 471, 472, 473, 474, 475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 486,
                489, 490, 493, 494, 495, 496, 497, 498, 499, 500, 501, 502, 503, 504, 505, 506,
                507, 508, 509, 510, 511, 512, 513, 514, 515, 516, 517, 518, 519, 520, 521, 522,
                523, 524, 526, 527, 528, 529, 530, 531, 532, 534, 535, 536, 537, 538, 539, 540,
                541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::PrintAction,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
                185, 187, 188, 189, 191, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205, 206,
                207, 208, 209, 210, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225, 228, 229,
                230, 231, 232, 233, 234, 235, 237, 238, 239, 240, 241, 245, 246, 247, 248, 249,
                250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263, 264, 265, 266,
                267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282,
                283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298,
                299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317,
                318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333,
                334, 335, 336, 337, 338, 339, 340, 341, 342, 344, 345, 346, 347, 348, 349, 350,
                351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365, 366, 367, 368,
                369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 384, 386, 387,
                388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403,
                404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 421,
                422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440, 441, 442, 443,
                444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458, 459, 460,
                461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476,
                477, 478, 479, 480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497, 498, 499,
                500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515,
                516, 517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530, 531, 532,
                534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Query,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 72, 73, 81,
                82, 87, 89, 90, 91, 95, 96, 97, 101, 105, 106, 111, 118, 120, 122, 125, 128, 129,
                132, 133, 134, 135, 137, 138, 139, 144, 145, 148, 149, 150, 151, 153, 154, 155,
                156, 160, 161, 162, 164, 165, 169, 172, 178, 179, 180, 182, 184, 186, 194, 195,
                196, 197, 198, 199, 200, 210, 212, 213, 214, 217, 222, 227, 229, 237, 245, 261,
                265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 283, 287, 297, 302,
                304, 305, 313, 315, 323, 325, 326, 327, 328, 329, 330, 333, 336, 337, 338, 341,
                357, 358, 359, 360, 381, 402, 407, 414, 417, 422, 423, 424, 427, 428, 429, 432,
                433, 442, 445, 455, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467,
                468, 469, 470, 471, 472, 473, 474, 475, 477, 489, 494, 521, 522, 523, 524, 531,
                532, 533,
            ],
        );
        map.insert(
            BuiltInAction::Run,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
                185, 187, 188, 189, 191, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205, 206,
                207, 208, 209, 210, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225, 228, 229,
                230, 231, 232, 233, 234, 235, 237, 238, 239, 240, 241, 245, 246, 247, 248, 249,
                250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263, 264, 265, 266,
                267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282,
                283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298,
                299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317,
                318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333,
                334, 335, 336, 337, 338, 339, 340, 341, 342, 344, 345, 346, 347, 348, 349, 350,
                351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365, 366, 367, 368,
                369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 384, 386, 387,
                388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403,
                404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 421,
                422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440, 442, 443, 444,
                445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458, 459, 460, 461,
                462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476, 477,
                478, 479, 480, 481, 482, 483, 484, 485, 486, 489, 490, 493, 494, 497, 498, 499,
                500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515,
                516, 517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530, 531, 532,
                534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Shutdown,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 81, 82, 87,
                89, 90, 91, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133, 134, 135,
                137, 139, 144, 145, 148, 149, 150, 151, 153, 160, 161, 162, 164, 165, 169, 178,
                180, 182, 194, 212, 213, 214, 217, 229, 237, 245, 265, 266, 267, 268, 269, 270,
                271, 272, 273, 274, 275, 276, 283, 287, 297, 305, 313, 315, 323, 325, 328, 330,
                333, 336, 337, 338, 341, 357, 358, 359, 360, 381, 382, 407, 414, 417, 422, 432,
                442, 445, 477, 489, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map.insert(
            BuiltInAction::Sync,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 33, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 73, 81,
                82, 87, 89, 90, 91, 95, 96, 97, 101, 105, 106, 111, 118, 125, 128, 129, 132, 133,
                134, 135, 137, 139, 144, 145, 148, 149, 150, 151, 153, 155, 156, 160, 161, 162,
                164, 165, 169, 178, 179, 180, 182, 194, 196, 197, 198, 199, 200, 210, 212, 213,
                214, 217, 229, 237, 245, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275,
                276, 283, 287, 297, 302, 304, 305, 313, 315, 323, 325, 326, 327, 328, 329, 330,
                333, 336, 337, 338, 341, 357, 358, 359, 360, 381, 402, 407, 414, 417, 422, 426,
                432, 433, 442, 445, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467,
                468, 469, 470, 471, 472, 473, 474, 475, 477, 489, 494, 521, 522, 523, 524, 531,
                532,
            ],
        );
        map.insert(
            BuiltInAction::Test,
            vec![
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
                69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
                109, 110, 111, 112, 113, 115, 116, 118, 119, 125, 126, 127, 128, 129, 130, 131,
                132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148,
                149, 150, 151, 152, 153, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165,
                166, 167, 168, 169, 171, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
                185, 187, 188, 189, 191, 192, 193, 194, 196, 197, 198, 199, 200, 203, 204, 205,
                206, 207, 208, 209, 210, 212, 213, 214, 217, 219, 220, 221, 223, 224, 225, 226,
                228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 245, 246,
                247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 262, 263,
                264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279,
                280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295,
                296, 297, 298, 299, 301, 302, 304, 305, 307, 308, 309, 310, 311, 312, 313, 314,
                315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330,
                331, 332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 344, 345, 346, 347,
                348, 349, 350, 351, 352, 353, 354, 356, 357, 358, 359, 360, 361, 363, 364, 365,
                366, 367, 368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381,
                384, 386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400,
                401, 402, 403, 404, 405, 406, 407, 409, 410, 411, 412, 413, 414, 415, 416, 417,
                418, 419, 421, 422, 425, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440,
                442, 443, 444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 456, 457, 458,
                459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474,
                475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 486, 489, 490, 493, 494, 497,
                498, 499, 500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513,
                514, 515, 516, 517, 518, 519, 520, 521, 522, 523, 524, 526, 527, 528, 529, 530,
                531, 532, 534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544,
            ],
        );
        map.insert(
            BuiltInAction::Version,
            vec![
                5, 9, 11, 12, 14, 15, 16, 17, 40, 45, 49, 50, 51, 53, 58, 62, 69, 71, 81, 82, 87,
                89, 90, 91, 96, 97, 101, 105, 106, 111, 117, 118, 125, 128, 129, 132, 133, 134,
                135, 137, 139, 144, 145, 148, 149, 150, 151, 153, 160, 161, 162, 164, 165, 169,
                178, 180, 182, 194, 212, 213, 214, 217, 229, 237, 245, 265, 266, 267, 268, 269,
                270, 271, 272, 273, 274, 275, 276, 283, 287, 297, 305, 313, 315, 323, 325, 328,
                330, 333, 336, 337, 338, 341, 357, 358, 359, 360, 381, 407, 414, 417, 422, 432,
                442, 445, 477, 489, 494, 521, 522, 523, 524, 531, 532,
            ],
        );
        map
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltInAction {
    AnalyzeProfile,
    Aquery,
    Build,
    CanonicalizeFlags,
    Clean,
    Coverage,
    Cquery,
    Dump,
    Fetch,
    Help,
    Info,
    License,
    MobileInstall,
    PrintAction,
    Query,
    Run,
    Shutdown,
    Sync,
    Test,
    Version,
}

use std::str::FromStr;
impl FromStr for BuiltInAction {
    type Err = ();

    fn from_str(input: &str) -> Result<BuiltInAction, Self::Err> {
        match input {
            "analyze-profile" => Ok(BuiltInAction::AnalyzeProfile),
            "aquery" => Ok(BuiltInAction::Aquery),
            "build" => Ok(BuiltInAction::Build),
            "canonicalize-flags" => Ok(BuiltInAction::CanonicalizeFlags),
            "clean" => Ok(BuiltInAction::Clean),
            "coverage" => Ok(BuiltInAction::Coverage),
            "cquery" => Ok(BuiltInAction::Cquery),
            "dump" => Ok(BuiltInAction::Dump),
            "fetch" => Ok(BuiltInAction::Fetch),
            "help" => Ok(BuiltInAction::Help),
            "info" => Ok(BuiltInAction::Info),
            "license" => Ok(BuiltInAction::License),
            "mobile-install" => Ok(BuiltInAction::MobileInstall),
            "print_action" => Ok(BuiltInAction::PrintAction),
            "query" => Ok(BuiltInAction::Query),
            "run" => Ok(BuiltInAction::Run),
            "shutdown" => Ok(BuiltInAction::Shutdown),
            "sync" => Ok(BuiltInAction::Sync),
            "test" => Ok(BuiltInAction::Test),
            "version" => Ok(BuiltInAction::Version),
            _ => Err(()),
        }
    }
}
impl core::fmt::Display for BuiltInAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuiltInAction::AnalyzeProfile => Ok(write!(f, "analyze-profile")?),
            BuiltInAction::Aquery => Ok(write!(f, "aquery")?),
            BuiltInAction::Build => Ok(write!(f, "build")?),
            BuiltInAction::CanonicalizeFlags => Ok(write!(f, "canonicalize-flags")?),
            BuiltInAction::Clean => Ok(write!(f, "clean")?),
            BuiltInAction::Coverage => Ok(write!(f, "coverage")?),
            BuiltInAction::Cquery => Ok(write!(f, "cquery")?),
            BuiltInAction::Dump => Ok(write!(f, "dump")?),
            BuiltInAction::Fetch => Ok(write!(f, "fetch")?),
            BuiltInAction::Help => Ok(write!(f, "help")?),
            BuiltInAction::Info => Ok(write!(f, "info")?),
            BuiltInAction::License => Ok(write!(f, "license")?),
            BuiltInAction::MobileInstall => Ok(write!(f, "mobile-install")?),
            BuiltInAction::PrintAction => Ok(write!(f, "print_action")?),
            BuiltInAction::Query => Ok(write!(f, "query")?),
            BuiltInAction::Run => Ok(write!(f, "run")?),
            BuiltInAction::Shutdown => Ok(write!(f, "shutdown")?),
            BuiltInAction::Sync => Ok(write!(f, "sync")?),
            BuiltInAction::Test => Ok(write!(f, "test")?),
            BuiltInAction::Version => Ok(write!(f, "version")?),
        }
    }
}
