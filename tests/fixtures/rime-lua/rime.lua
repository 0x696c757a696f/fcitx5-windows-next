function fcitx_windows_lua_probe(input, segment)
  if input == "zzlua" then
    yield(Candidate("fcitx_windows_test", segment.start, segment._end,
                    "Lua Works", "rime-lua"))
  end
end
