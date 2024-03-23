#include "sqlite-sample.h"

#include "sqlite3ext.h"
SQLITE_EXTENSION_INIT1

static void sample(sqlite3_context *context, int argc, sqlite3_value **argv) {
  sqlite3_result_text(
      context,
      "yo!",
      -1,
      SQLITE_STATIC
  );
}
static void sample_version(sqlite3_context *context, int argc, sqlite3_value **argv) {
  sqlite3_result_text(context, SQLITE_SAMPLE_VERSION, -1, SQLITE_STATIC);
}

#ifdef _WIN32
__declspec(dllexport)
#endif
int sqlite3_sample_init(sqlite3 *db, char **pzErrMsg, const sqlite3_api_routines *pApi) {
  SQLITE_EXTENSION_INIT2(pApi);
  int rc = SQLITE_OK;
  if(rc == SQLITE_OK) rc = sqlite3_create_function_v2(db, "sample", 0, SQLITE_UTF8 | SQLITE_DETERMINISTIC, 0, sample, 0, 0, 0);
  if(rc == SQLITE_OK) rc = sqlite3_create_function_v2(db, "sample_version", 0, SQLITE_UTF8 | SQLITE_DETERMINISTIC, 0, sample_version, 0, 0, 0);
  return rc;
}
