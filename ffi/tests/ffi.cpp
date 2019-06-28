
#include <cassert>
#include <cstdbool>
#include <cstdio>
#include <cstdint>
#include <cstring>

#include <thread>
#include <vector>

#include "loot_condition_interpreter.h"

void test_game_id_values() {
  printf("testing LCI_GAME_* values...\n");
  assert(LCI_GAME_MORROWIND == 8);
  assert(LCI_GAME_OBLIVION == 0);
  assert(LCI_GAME_SKYRIM == 1);
  assert(LCI_GAME_SKYRIM_SE == 2);
  assert(LCI_GAME_SKYRIM_VR == 3);
  assert(LCI_GAME_FALLOUT_3 == 4);
  assert(LCI_GAME_FALLOUT_NV == 5);
  assert(LCI_GAME_FALLOUT_4 == 6);
  assert(LCI_GAME_FALLOUT_4_VR == 7);
}

void test_lci_condition_parse() {
    printf("testing lci_condition_parse()...\n");

    int return_code = lci_condition_parse("file(\"Blank.esm\")");

    assert(return_code == LCI_OK);
}

void test_lci_get_error_message() {
    printf("testing lci_get_error_message()...\n");

    const char * message = nullptr;
    int return_code = lci_get_error_message(&message);

    assert(return_code == LCI_OK);
    assert(message == nullptr);

    return_code = lci_condition_parse("file(\"Blank.");

    assert(return_code == LCI_ERROR_PARSING_ERROR);

    return_code = lci_get_error_message(&message);
    assert(return_code == LCI_OK);
    assert(message != nullptr);
    assert(strcmp(message, "An error was encountered while parsing the expression \"file(\\\"Blank.\": Error in parser: Separated list") == 0);
}

void test_lci_state_create() {
    printf("testing lci_state_create()...\n");

    lci_state * state = nullptr;
    int return_code = lci_state_create(&state, LCI_GAME_OBLIVION, ".", ".");

    assert(return_code == LCI_OK);
    assert(state != nullptr);

    lci_state_destroy(state);
}

void test_lci_condition_eval() {
    printf("testing lci_condition_eval()...\n");

    lci_state * state = nullptr;
    int return_code = lci_state_create(&state, LCI_GAME_OBLIVION, "../../tests/testing-plugins/Oblivion/Data", ".");

    assert(return_code == LCI_OK);
    assert(state != nullptr);

    return_code = lci_condition_eval("file(\"Blank.esm\")", state);

    assert(return_code == LCI_RESULT_TRUE);

    return_code = lci_condition_eval("file(\"missing.esm\")", state);

    assert(return_code == LCI_RESULT_FALSE);

    lci_state_destroy(state);
}

void test_lci_state_set_active_plugins() {
    printf("testing lci_state_set_active_plugins()...\n");

    lci_state * state = nullptr;
    int return_code = lci_state_create(&state, LCI_GAME_OBLIVION, "../../tests/testing-plugins/Oblivion/Data", ".");

    assert(return_code == LCI_OK);
    assert(state != nullptr);

    char const * plugins[] = { "Blank.esm" };

    return_code = lci_state_set_active_plugins(state, plugins, 0);
    assert(return_code != LCI_OK);

    return_code = lci_state_set_active_plugins(state, nullptr, 1);
    assert(return_code != LCI_OK);

    return_code = lci_state_set_active_plugins(state, plugins, 1);
    assert(return_code == LCI_OK);

    return_code = lci_condition_eval("active(\"Blank.esm\")", state);
    assert(return_code == LCI_RESULT_TRUE);

    return_code = lci_state_set_active_plugins(state, nullptr, 0);
    assert(return_code == LCI_OK);

    return_code = lci_condition_eval("active(\"Blank.esm\")", state);
    assert(return_code == LCI_RESULT_FALSE);

    lci_state_destroy(state);
}

void test_lci_state_set_plugin_versions() {
    printf("testing lci_state_set_plugin_versions()...\n");

    lci_state * state = nullptr;
    int return_code = lci_state_create(&state, LCI_GAME_OBLIVION, "../../tests/testing-plugins/Oblivion/Data", ".");

    assert(return_code == LCI_OK);
    assert(state != nullptr);

    plugin_version plugins[] = { {"Blank.esm", "5"} };

    return_code = lci_state_set_plugin_versions(state, plugins, 0);
    assert(return_code != LCI_OK);

    return_code = lci_state_set_plugin_versions(state, nullptr, 1);
    assert(return_code != LCI_OK);

    return_code = lci_state_set_plugin_versions(state, plugins, 1);
    assert(return_code == LCI_OK);

    return_code = lci_condition_eval("version(\"Blank.esm\", \"5\", ==)", state);
    assert(return_code == LCI_RESULT_TRUE);

    return_code = lci_state_set_plugin_versions(state, nullptr, 0);
    assert(return_code == LCI_OK);

    return_code = lci_state_clear_condition_cache(state);
    assert(return_code == LCI_OK);

    return_code = lci_condition_eval("version(\"Blank.esm\", \"5\", ==)", state);
    assert(return_code == LCI_RESULT_FALSE);

    lci_state_destroy(state);
}

void test_lci_state_set_crc_cache() {
    printf("testing lci_state_set_crc_cache()...\n");

    lci_state * state = nullptr;
    int return_code = lci_state_create(&state, LCI_GAME_OBLIVION, "../../tests/testing-plugins/Oblivion/Data", ".");

    assert(return_code == LCI_OK);
    assert(state != nullptr);

    plugin_crc plugin_crcs[] = { {"Blank.esm", 0xDEADBEEF} };

    return_code = lci_state_set_crc_cache(state, plugin_crcs, 0);
    assert(return_code != LCI_OK);

    return_code = lci_state_set_crc_cache(state, nullptr, 1);
    assert(return_code != LCI_OK);

    return_code = lci_state_set_crc_cache(state, plugin_crcs, 1);
    assert(return_code == LCI_OK);

    return_code = lci_condition_eval("checksum(\"Blank.esm\", DEADBEEF)", state);
    assert(return_code == LCI_RESULT_TRUE);

    return_code = lci_state_set_crc_cache(state, nullptr, 0);
    assert(return_code == LCI_OK);

    return_code = lci_condition_eval("checksum(\"Blank.esm\", DEADBEEF)", state);
    assert(return_code == LCI_RESULT_FALSE);

    lci_state_destroy(state);
}

int main(void) {
    test_game_id_values();

    test_lci_condition_parse();
    test_lci_get_error_message();

    test_lci_state_create();
    test_lci_condition_eval();
    test_lci_state_set_active_plugins();
    test_lci_state_set_plugin_versions();
    test_lci_state_set_crc_cache();

    printf("SUCCESS\n");
    return 0;
}
