file-prefix search utils for my usual tasks

if needs to add new search pattern, add config in `~/.config/prefix-search/config.toml` like below:

```toml
[video]
dirs = ["/home/username/Videos", "/mnt/another-disk/Videos"]
```

and use:

```sh
prefix-search video "video-prefix"
```

