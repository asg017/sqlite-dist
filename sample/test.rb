require 'sqlite3'
require 'sqlite_sample'
db = SQLite3::Database.new(':memory:')
db.enable_load_extension(true)
SqliteSample.load(db)
db.enable_load_extension(false)
result = db.execute('SELECT sample_version()')
puts result.first.first

