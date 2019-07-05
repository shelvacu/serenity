ARGV.each do |fn|
  data = File.read(fn)
  data.gsub!(/^(\s*)[a-zA-Z0-9]*(::)?__Nonexhaustive/) do |s|
    $1 + '#[cfg(not(feature = "allow_exhaustive_enum"))]' + "\n" + s
  end
  File.rename(fn, fn+"~")
  File.write(fn, data)
  File.unlink(fn+"~")
end
