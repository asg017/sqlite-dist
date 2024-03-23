#ifndef _SQLITE_SAMPLE_H
#define _SQLITE_SAMPLE_H

#include "sqlite3ext.h"

#define SQLITE_SAMPLE_VERSION "v0.0.1-alpha.1"
#define SQLITE_SAMPLE_DATE "2024-02-24T22:49:08Z-0800"
#define SQLITE_SAMPLE_SOURCE ""

#ifdef __cplusplus
extern "C" {
#endif

int sqlite3_sample_init(sqlite3*, char**, const sqlite3_api_routines*);

#ifdef __cplusplus
}  /* end of the 'extern "C"' block */
#endif

#endif /* ifndef _SQLITE_SAMPLE_H */
