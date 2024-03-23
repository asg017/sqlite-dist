import { Database } from "jsr:@db/sqlite@0.11";
import * as sqlite_sample from "sqlite-sample";
import * as sqlite_hello from "npm:sqlite-hello";

const db = new Database(":memory:");
db.enableLoadExtension = true;
sqlite_sample.load(db);
db.loadExtension(sqlite_hello.getLoadablePath());

const version = db.prepare("select hello_version()").value();
//const version = db.prepare("select sample_version()").value();
//const version = db.prepare("select sample_version()").pluck().get();
console.log(version);
