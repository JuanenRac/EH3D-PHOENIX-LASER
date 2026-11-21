# EH3D-PHOENIX-LASER: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because nothing of this program has run on a real laser, a real material or a real server: the G-code is checked by the program's own reader, the server is a stand-in, and the pictures for the marks are simulated.

The register has 95 requirements: 85 implemented, 5 prototype, 5 planned and 0 validated. By progress: 85 done, 5 partial, 1 pending and 4 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `PLAS-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a stand-in of the EH3D server or a simulated picture of the marks, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Pictures and dots

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| PLAS-001 | A PGM is read in both forms with comments and a damaged one is an error. | implemented | done | host-test | `tests/laser::a_pgm_is_read_in_both_forms_with_comments` | - |
| PLAS-002 | A PNG in grey, colour or with transparency becomes grey levels. | implemented | done | host-test | `tests/laser::a_png_in_grey_colour_and_transparency_becomes_grey_levels` | - |
| PLAS-003 | Error diffusion keeps the darkness of the picture. | implemented | done | host-test | `tests/laser::error_diffusion_keeps_the_darkness_of_the_picture` | - |
| PLAS-004 | Ordered dithering keeps the darkness of a grey and repeats every eight dots. | implemented | done | host-test | `src/dither::bayer_keeps_the_darkness_of_a_grey` | - |
| PLAS-005 | The tone curve changes nothing by default and gamma keeps black and white. | implemented | done | host-test | `src/tone::gamma_moves_the_middle_and_keeps_black_and_white` | - |
| PLAS-006 | A threshold burns what is darker and its extremes are none and almost all. | implemented | done | host-test | `tests/more::the_threshold_burns_strictly_darker_and_its_extremes_are_none_and_almost_all` | - |
| PLAS-007 | A row of dots becomes runs with the power on only where it burns. | implemented | done | host-test | `tests/laser::a_row_of_dots_becomes_runs_with_the_power_on_only_where_it_burns` | - |
| PLAS-008 | The overscan runs the head past the work with the laser off. | implemented | done | host-test | `tests/laser::the_overscan_runs_the_head_past_the_work_with_the_laser_off` | - |
| PLAS-009 | The run-up is what the acceleration needs. | implemented | done | host-test | `tests/laser::the_run_up_is_what_the_acceleration_needs` | - |
| PLAS-010 | The laser is off whenever the head moves without burning, and a rapid move between rows has the power at zero. | implemented | done | host-test | `tests/rapid_power::a_rapid_move_between_rows_of_an_image_has_the_power_at_zero_even_without_overscan` | - |
| PLAS-011 | The program always starts with the units and the absolute mode and ends with the laser off. | implemented | done | host-test | `tests/more2::the_program_always_starts_with_the_units_the_absolute_mode_and_dynamic_power_and_ends_with_the_laser_off` | - |
| PLAS-012 | The same dots give the same G-code byte for byte. | implemented | done | host-test | `tests/more::the_same_dots_give_the_same_gcode_byte_for_byte` | - |
| PLAS-013 | A number that is not finite never reaches the file. | implemented | done | host-test | `tests/more::a_number_that_is_not_finite_never_reaches_the_file` | - |
| PLAS-014 | A dry run counts the same and writes nothing. | implemented | done | host-test | `tests/dry_run::a_dry_run_counts_the_same_and_writes_nothing` | - |
| PLAS-015 | The engraving never writes over the image it is made from. | implemented | done | host-test | `tests/safe_output::engrave_never_writes_over_the_image_it_is_made_from` | - |
| PLAS-016 | The contours of a picture are followed by marching squares and cut like the shapes of a drawing. | implemented | done | host-test | `src/trace::a_disc_becomes_one_closed_smooth_ring_of_the_right_size` | - |
| PLAS-017 | The calibration grid has a square for every pair of power and feed. | implemented | done | host-test | `tests/laser::the_calibration_grid_has_a_square_for_every_pair` | - |
| PLAS-018 | The focus test burns one line for each height, goes back, and is refused without a step. | implemented | done | host-test | `src/calibrate::without_a_step_the_test_is_refused` | - |

## Drawings and cutting

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| PLAS-019 | An SVG path with a size and a view box is read in millimetres. | implemented | done | host-test | `src/svg::a_closed_path_with_a_size_and_a_view_box_is_read_in_millimetres` | - |
| PLAS-020 | Entities of an SVG are never expanded, and a use that leads back to itself is an error. | implemented | done | host-test | `src/svg::entities_are_never_expanded_and_declaring_one_is_an_error` | - |
| PLAS-021 | A DXF circle, arc, ellipse and spline become shapes in millimetres. | implemented | done | host-test | `src/dxf::a_circle_is_a_closed_shape_of_the_right_size` | - |
| PLAS-022 | A DXF insert draws its block at its place, scale and rotation. | implemented | done | host-test | `src/dxf::an_insert_draws_the_block_at_its_place_scale_and_rotation` | - |
| PLAS-023 | A file cut at any place or made of garbage never panics the readers. | implemented | done | host-test | `src/dxf::a_file_cut_at_any_place_or_made_of_garbage_never_panics` | - |
| PLAS-024 | LBRN2 projects are read: rectangles, paths, groups, transforms and the outline of a text become a drawing, and the layers fill the table. | prototype | partial | host-test | `tests/lbrn2::the_layers_are_read_with_their_type_number_name_and_every_value_as_written` | Measured on twelve real projects of one laser maker; a picture inside a project is not read, and the orientation of the y axis and the rounded corner of a rectangle are not confirmed on a machine. |
| PLAS-025 | A project is cut into a program that passes the check. | implemented | done | host-test | `tests/lbrn2::a_project_is_cut_into_a_program_that_passes_the_check` | - |
| PLAS-026 | A declared entity of a project is never expanded and damaged projects never stop the program. | implemented | done | host-test | `tests/lbrn2::a_declared_entity_is_never_expanded` | - |
| PLAS-027 | Curves are flattened within the tolerance. | implemented | done | host-test | `src/flatten::a_flattened_circle_keeps_every_point_on_the_circle_and_every_piece_within_the_tolerance` | - |
| PLAS-028 | The contour inside another is cut first, even when the other is nearer. | implemented | done | host-test | `src/order::a_contour_is_cut_before_the_one_it_lies_inside_even_when_that_one_is_nearer` | - |
| PLAS-029 | Two-opt shortens the trip and never breaks the inside-first rule. | implemented | done | host-test | `src/order::two_opt_never_breaks_the_inside_first_rule` | - |
| PLAS-030 | The beam goes outside a piece and inside a hole. | implemented | done | host-test | `src/cut::the_width_of_the_beam_goes_outside_a_piece_and_inside_a_hole` | - |
| PLAS-031 | A tab leaves the middle of the outline uncut. | implemented | done | host-test | `src/cut::a_tab_leaves_the_middle_of_the_outline_uncut` | - |
| PLAS-032 | Several passes move the focus and put it back. | implemented | done | host-test | `src/cut::several_passes_move_the_focus_and_put_it_back` | - |
| PLAS-033 | The hatch fills a shape with lines that go one way and the other, and a cross hatch fills it a second time. | implemented | done | host-test | `src/cut::the_hatch_fills_a_square_with_lines_that_go_one_way_and_the_other` | - |
| PLAS-034 | Work that goes outside of the bed is an error. | implemented | done | host-test | `src/cut::work_that_goes_outside_of_the_bed_is_an_error` | - |
| PLAS-035 | The offset of a ring is exact for a square and never panics on random rings. | implemented | done | host-test | `src/offset::random_rings_never_panic_and_what_is_given_is_a_counterclockwise_ring` | - |
| PLAS-036 | A drawing goes from DXF to SVG and back without losing a shape. | implemented | done | host-test | `src/export::a_drawing_goes_from_dxf_to_svg_and_back_without_losing_a_shape` | - |
| PLAS-037 | The same job twice gives the same program. | implemented | done | host-test | `tests/vector::the_same_job_twice_gives_the_same_program` | - |
| PLAS-038 | Hostile drawings are refused without a panic. | implemented | done | host-test | `tests/vector::hostile_files_are_refused_without_a_panic` | - |
| PLAS-039 | Read the pictures inside an LBRN2 project and the hatches, text and dimensions of a DXF. | planned | pending | none | `planned:readers::a_picture_inside_a_project_and_a_dxf_hatch_are_read` | Not written: they are skipped with a note today. |

## Text, codes, marks and the frame

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| PLAS-040 | Text is burnt with a font of strokes at the power and speed asked and the program passes its own check. | implemented | done | host-test | `src/text::the_program_burns_every_stroke_at_the_power_and_speed_asked_and_passes_the_check` | - |
| PLAS-041 | A letter the font has not is refused and the text is never written into a comment. | implemented | done | host-test | `src/text::the_text_is_never_written_into_a_comment_of_the_program` | - |
| PLAS-042 | A QR code of the program's own reads back in every version and level and follows the standard. | prototype | partial | host-test | `src/qr::every_version_level_and_length_reads_back_what_was_put_in` | Read back by readers of the program's own and by an independent decoder once; no scanner has read a symbol burnt on a material. |
| PLAS-043 | The error correction codewords and the capacity of the QR code are the ones of the standard. | implemented | done | host-test | `src/qr::the_error_correction_codewords_are_the_ones_of_the_standard` | - |
| PLAS-044 | A Code 128 barcode reads back with the check symbol of the standard. | prototype | partial | host-test | `src/barcode::everything_that_can_be_written_reads_back` | Read back by the tests' own reader; no scanner has read a burnt barcode. |
| PLAS-045 | A burning that would take longer than the limit is refused and never shortened. | implemented | done | host-test | `src/text::a_burning_that_would_take_longer_than_the_limit_is_refused_never_shortened` | - |
| PLAS-046 | The positioning marks are an L at each corner and burn only there. | implemented | done | host-test | `src/frame::the_marks_are_an_l_at_each_corner_on_the_edges_and_burn_only_there` | - |
| PLAS-047 | The frame with the laser off never sets a power or turns the spindle on. | implemented | done | host-test | `src/frame::the_frame_with_the_laser_off_never_sets_a_power_or_turns_the_spindle_on` | - |
| PLAS-048 | A pilot power needs the confirmation every time and stays small. | implemented | done | host-test | `tests/laser_tools::a_pilot_power_needs_the_confirmation_and_stays_small` | - |
| PLAS-049 | The shift and the turn of the material are found again from the picture of the marks. | prototype | partial | simulator | `src/register::a_shift_of_the_material_is_found_again_to_a_fraction_of_a_millimetre` | Tried on simulated pictures with fixed seeds; no camera has seen a real mark. |
| PLAS-050 | Align a job on a real material with a real camera. | planned | blocked | none | `planned:camera::a_real_picture_of_the_marks_gives_the_shift_of_the_material` | Needs the vision tool of the suite and a camera over the bed. |

## Safety, limits and the check of a program

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| PLAS-051 | A power over the limit is refused and one at the limit is not. | implemented | done | host-test | `src/safety::a_power_over_the_limit_is_refused_and_one_at_the_limit_is_not` | - |
| PLAS-052 | A speed over the limit is refused and never lowered. | implemented | done | host-test | `tests/laser_tools::a_feed_over_the_limit_of_speed_is_refused_and_never_lowered` | - |
| PLAS-053 | A power over the limit of power is refused in the command and one at the limit is not. | implemented | done | host-test | `tests/laser_tools::a_power_over_the_limit_of_power_is_refused_and_one_at_the_limit_is_not` | - |
| PLAS-054 | A good program passes its check and its hash is the one of its bytes. | implemented | done | host-test | `src/verify::a_good_program_passes_and_its_hash_is_the_one_of_its_bytes` | - |
| PLAS-055 | The laser on during a rapid move, and a burning move with no feed, fail the check. | implemented | done | host-test | `src/verify::the_laser_on_during_a_rapid_move_and_a_burning_move_with_no_feed_fail` | - |
| PLAS-056 | Work outside the bed fails the check whatever the allowance. | implemented | done | host-test | `src/verify::burning_outside_the_bed_fails_whatever_the_allowance` | - |
| PLAS-057 | A program that is not the one that was authorised fails the check by its hash. | implemented | done | host-test | `tests/laser_tools::verify_fails_a_program_that_is_not_the_one_that_was_authorised` | - |
| PLAS-058 | Whatever the settings, the program passes its own safety check or is not given. | implemented | done | host-test | `tests/vector::whatever_the_settings_the_program_passes_its_own_safety_check_or_is_not_given` | - |
| PLAS-059 | A program that burns in a no-burn zone fails and a no-go zone is not crossed. | implemented | done | host-test | `src/guard::a_no_burn_zone_lets_the_head_cross_with_the_laser_off_and_a_no_go_zone_does_not` | - |
| PLAS-060 | A pressed emergency stop stops everything and an interlock that is not known is not safe. | implemented | done | host-test | `src/guard::a_pressed_emergency_stop_stops_everything` | - |
| PLAS-061 | A required interlock must be in its safe state and a state not known is not safe. | implemented | done | host-test | `src/guard::a_required_interlock_must_be_in_its_safe_state_and_a_state_not_known_is_not_safe` | - |
| PLAS-062 | With nothing said, the interlocks stop nothing. | implemented | done | host-test | `src/guard::with_nothing_said_the_interlocks_stop_nothing` | - |
| PLAS-063 | The stop and the interlocks are outside every panel so no workspace hides them. | implemented | done | host-test | `src/panels::the_stop_and_the_interlocks_are_outside_every_panel_so_no_workspace_hides_them` | - |
| PLAS-064 | The preview reads a program as text and notes what looks dangerous. | implemented | done | host-test | `src/preview::the_things_that_look_dangerous_are_noted` | - |
| PLAS-065 | The report says the verdict, the hash, the material, the zones and the interlocks in the eight languages. | implemented | done | host-test | `src/report::the_report_of_a_good_program_gives_its_hash_its_numbers_and_a_pass_in_every_language` | - |
| PLAS-066 | A material over its own limit of power or speed is refused when the library is read. | implemented | done | host-test | `src/materials::a_material_whose_power_or_speed_is_over_its_own_limit_is_refused_when_the_library_is_read` | - |
| PLAS-067 | The starter library is all unverified and keeps to its own limits. | implemented | done | host-test | `src/materials::the_starter_library_is_all_unverified_and_keeps_to_its_own_limits` | - |
| PLAS-068 | A thickness outside what the database knows is refused and never extrapolated. | implemented | done | host-test | `src/commonmat::a_thickness_outside_what_is_known_is_refused_and_never_extrapolated` | - |
| PLAS-069 | A material that must not meet a laser is refused with the reasons. | implemented | done | host-test | `src/commonmat::a_material_that_must_not_meet_a_laser_is_refused_with_the_reasons` | - |
| PLAS-070 | Every default of the settings is accepted by the reader and every documented setting is read. | implemented | done | host-test | `tests/vector::every_setting_the_file_reads_is_documented_and_every_documented_setting_is_read` | - |
| PLAS-071 | A mistake of the settings file is an error that names the line. | implemented | done | host-test | `src/vconfig::a_mistake_is_an_error_that_names_the_line` | - |
| PLAS-072 | A profile is the machine and its limits hold over the layers and the defaults. | implemented | done | host-test | `src/vconfig::the_limits_of_the_profile_hold_over_the_layers_and_the_defaults` | - |
| PLAS-073 | The same work is written byte for byte every time. | implemented | done | host-test | `tests/fuzz::the_same_work_is_written_byte_for_byte_every_time` | - |
| PLAS-074 | Copies of a file with bytes changed, inserted or cut are read without a panic. | implemented | done | host-test | `tests/fuzz::copies_with_bytes_changed_inserted_or_cut_are_read_without_a_panic` | - |
| PLAS-075 | Show the power density on the board before burning and keep the power constant in the corners of a cut. | prototype | partial | host-test | `src/energy::a_line_gets_the_power_over_the_speed` | `layers` prints the energy of a pass when the power of the laser is known; the density on the board and the corners are not drawn. |
| PLAS-076 | Burn a real material on a real laser with a program that passed every check. | planned | blocked | none | `planned:real_laser::a_real_laser_burns_a_program_that_passed_the_check_and_stops_on_the_emergency_stop` | Needs a person at a real laser with the emergency stop in reach. |

## The server, the window and the languages

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| PLAS-077 | A program is sent to the server, started and followed to its end. | implemented | done | simulator | `src/server::a_program_is_sent_started_and_followed_to_its_end` | - |
| PLAS-078 | Nothing over forty percent of power leaves unless the file raises the limit of what is sent. | implemented | done | simulator | `src/server::nothing_over_forty_percent_leaves_unless_the_file_raises_the_limit_of_what_is_sent` | - |
| PLAS-079 | A program that reaches a forbidden zone or meets a pressed stop is not sent. | implemented | done | simulator | `src/server::a_program_that_reaches_a_forbidden_zone_or_meets_a_pressed_stop_is_not_sent` | - |
| PLAS-080 | The stop is an emergency stop that latches the machine and only an administrator releases it. | implemented | done | simulator | `src/server::the_stop_is_an_emergency_stop_that_latches_the_machine_and_only_an_administrator_releases_it` | - |
| PLAS-081 | A drawing is sent as a vector job with its layers and followed to its end. | implemented | done | simulator | `tests/server_cli::a_drawing_is_sent_as_a_vector_job_with_its_layers_and_followed_to_its_end` | - |
| PLAS-082 | A test fire needs the confirmation and the limits, and the server refuses what is over its own. | implemented | done | simulator | `tests/server_cli::a_test_fire_needs_the_confirmation_and_the_limits_and_the_server_refuses_what_is_over_its_own` | - |
| PLAS-083 | The token is never shown. | implemented | done | host-test | `src/linkpane::the_store_is_shared_by_its_clones_and_never_shows_the_token` | - |
| PLAS-084 | Send a job to a real server with a real laser registered on it. | planned | blocked | none | `planned:real_server::a_job_sent_to_a_real_server_reaches_a_real_laser` | The server is a stand-in in the tests; it needs the server deployed and a laser registered. |
| PLAS-085 | The window engraves like the command line. | implemented | done | host-test | `src/window::the_window_engraves_like_the_command_line` | - |
| PLAS-086 | The window refuses a power or a speed over the limit and never lowers it. | implemented | done | host-test | `src/window::a_power_or_a_speed_over_the_limit_is_refused_and_never_lowered` | - |
| PLAS-087 | A pilot power in the window asks for the confirmation every time. | implemented | done | host-test | `src/window::a_frame_that_lights_the_laser_needs_the_confirmation_of_the_person_every_time` | - |
| PLAS-088 | A program opened to be seen is never written again. | implemented | done | host-test | `src/window::a_program_is_opened_to_be_seen_and_is_never_written_again` | - |
| PLAS-089 | A power from a project over the limit of the machine is refused and never lowered. | implemented | done | host-test | `tests/dockwindow::a_power_from_a_project_over_the_limit_of_the_machine_is_refused_and_never_lowered` | - |
| PLAS-090 | The arrangement of the panels is written next to the settings only when it changes. | implemented | done | host-test | `tests/dockwindow::the_arrangement_is_written_next_to_the_settings_without_touching_them_and_only_when_it_changes` | - |
| PLAS-091 | A starting point of the database goes into the boxes as the database has it and stays unverified. | implemented | done | host-test | `tests/dockwindow::a_known_point_goes_into_the_boxes_and_the_table_as_the_database_has_it_and_stays_unverified` | - |
| PLAS-092 | Every sentence has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |
| PLAS-093 | Every command of the help has its line in the eight READMEs. | implemented | done | host-test | `tests/readme::every_command_of_the_help_has_its_line_in_the_eight_readmes` | - |
| PLAS-094 | A build without a window says so. | implemented | done | host-test | `tests/window_cli::the_gui_word_says_that_this_build_has_no_window` | - |
| PLAS-095 | A person looks at the window on a screen and finds it usable at a laser. | planned | blocked | none | `planned:operator::an_operator_prepares_and_runs_a_job_with_the_window` | Needs a person at a screen, at a laser. |
