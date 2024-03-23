ifndef TARGET_LOADABLE
TARGET_LOADABLE=sample0.dylib
endif

# -lsqlite3
CFLAGS=

ifdef TARGET
CFLAGS += -target $(TARGET)
endif

$(TARGET_LOADABLE): sqlite-sample.c
	zig cc \
		-fPIC -shared \
		-Wall -Wextra -Wno-unused-parameter -Wno-unused-variable \
		-Ivendor \
		-O3 \
		$(CFLAGS) \
		$< \
		-o $@

loadable: $(TARGET_LOADABLE)

$(TARGET_STATIC): sqlite-sample.c $(prefix)
	zig cc \
		-Wall -Wextra -Wno-unused-parameter -Wno-unused-variable \
		-Ivendor $(CFLAGS) -DSQLITE_CORE \
		-O3 -c  $< -o tmp.o
	zig ar rcs $@ tmp.o
	rm tmp.o

static: $(TARGET_STATIC)

sqlite-sample.h: sqlite-sample.h.tmpl VERSION
	VERSION=$(shell cat VERSION) \
	DATE=$(shell date -r VERSION +'%FT%TZ%z') \
	SOURCE=$(shell git log -n 1 --pretty=format:%H -- VERSION) \
		envsubst < $< > $@

$(TARGET_H): sqlite-sample.h
	cp $< $@

h: $(TARGET_H)

.PHONY: loadable static h
