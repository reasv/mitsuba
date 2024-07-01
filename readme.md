# Mitsuba
Mitsuba is a lightweight 4chan board archiver written in Rust. It continuously monitors a set of 4chan boards, fetches new posts, thumbnails, and optionally full images, and makes them available through an imageboard web UI as well as a read-only JSON API that is compatible with 4chan's [official API](https://github.com/4chan/4chan-API).

Mitsuba's main goal is to be very lightweight in terms of CPU and memory usage, and Rust helps accomplish this goal. Mitsuba is designed to be easy to deploy and doesn't currently have any runtime dependencies besides needing a Postgresql database.

The intended usage is self-hosting an archive on a low budget, however the Actix based web UI and API are quite performant, and should be capable of scaling to any amount of readers, with much lower resource consumption compared to competing frameworks in other languages, and without the possible latency spikes caused by garbage collection.

Mitsuba does not support "ghost posting" as it's not an imageboard engine. This could be supported in the future with some work (mostly on the front-end) but it requires actual administration tools and an accounts system, neither of which are actually present. What few options Mitsuba has are set through the CLI and environment variables (see `example.env`)

## Features
- Very quick and easy to set up
- No runtime dependencies except a running Postgresql database
- Single static executable, all assets and dependencies embedded
- Extremely lightweight, can run on a budget VPS
- Fully integrated: Mitsuba archives boards, threads, and images, serves them through a JSON API and Web UI all in one
- Easy administration with a few CLI commands
- Configurable rate limiter
- Optional full image download setting per-board
- Web UI has a field that lets you jump to any post by typing its ID and selecting the board
- Sha256 image deduplication, doesn't rely on 4chan's MD5 hash
- Support for S3-compatible image storage backend 
- Reduced database writes: the hash of every post is kept in memory, if a post hasn't changed, no DB operation is performed
- Can find an image from its original 4chan URL. `https://i.4cdn.org/po/1546293948883.png` can be found on mitsuba at `/po/1546293948883.png`
- Can be configured to load balance requests to 4chan between multiple proxies with different weights, to bypass rate limits

There are some important features missing:
- No "ghost posting" or posting of any kind. Read only archive.
- No full text search, or any search really. Can only get to a post or thread from the ID. We want to have search eventually.
- No admin UI, administration CLI only (but there are only a few things you'd want to change anyways)
- No administration tools to delete individual posts or images (but you can safely delete an image file from the folder if necessary, it won't be downloaded again)
- No account system whatsoever (but this makes it inherently secure)
- No tools for deleting all posts or images from a particular board (This is a planned feature, for now you could just run one instance per board)

## Dependencies
You need to have a Postgresql instance available somewhere Mitsuba can reach it with the DATABASE_URL env variable provided.
If you get an error about the server not accepting any more connections on startup, you might need to increase your database's `max_connections` configuration.

## Quick Setup
```
export DATABASE_URL="postgres://user:password@127.0.0.1/mitsuba"
export RUST_LOG=mitsuba=info # Optional, to get feedback
mitsuba add po
mitsuba start
```
After some threads have been archived, you can visit http://127.0.0.1:8080/po/1 to see your new archive for the /po/ board.

This will only get posts and thumbnails but not full images.

Use `mitsuba add po --full-images=true` to change that.

Mitsuba will not fetch full images for a post it has already archived previously, unless it has to visit that post again. Currently this means if you enable full images on a board you were already archiving with only thumbnails, Mitsuba won't fetch full images for posts on threads until that particular thread gets a new post.

`mitsuba add BOARD` and `mitsuba remove BOARD` are safe to use while mitsuba is running. But in that scenario, they will not take effect until the current archive cycle is completed.

## Setup Guide
Mitsuba is designed to be easy and quick to set up.
Download a binary build for your system, or clone the repository and build your executable.
Currently all static files mitsuba uses are embedded in the executable. You should just be able to run Mitsuba in an empty folder.

Some options need to be passed as environment variables. Mitsuba uses `dotenv`, which means that instead of setting the environment variables yourself, you can specify their values in a file called `.env` which must be in the directory you are running the mitsuba executable in. Mitsuba will read this file and apply the environment variables specified with their values.

You will find an `example.env` file in this repository. Copy it and rename it to just `.env`, then edit its configuration as needed.
There are a couple of settings that you need to be aware of right now:

- DATABASE_URL: you need to specify the connection URI for your Postgresql instance here. In the example file it's `postgres://user:password@127.0.0.1/mitsuba` .
   Replace with the correct username and password, as well as address, port and database name. The user needs to either be the owner of the specified database (in this case called `mitsuba`) if it already exists, or you need to create it yourself.

- DATA_ROOT: the directory in which to save the image files. This is an optional setting. If you leave it out entirely, Mitsuba will just create a "data" folder in the current working directory, and use that.

- RUST_LOG: the recommended setting for this is "mitsuba=info". This controls the mitsuba's log output. For now mitsuba uses `env_logger` which just prints the log information to standard output (the console). If this is not set, you will only see errors and warnings and no other feedback. 
  
The only required setting is DATABASE_URL but RUST_LOG="info" is also recommended. Just use the `example.env` file contents and change the database URI.

We will refer to the executable as `mitsuba` in this guide from now on, but on Windows® it is of course called `mitsuba.exe` .

Run `mitsuba help` to get a quick usage guide with a list of possible commands:

```
$ mitsuba help
mitsuba 1.0
High performance board archiver software. Add boards with `add`, then start with `start` command.
See `help add` and `help start`

USAGE:
    mitsuba.exe <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add                Add a board to the archiver, or replace its settings.
    help               Prints this message or the help of the given subcommand(s)
    list               List all boards in the database and their current settings. Includes
                       disabled ('removed') boards
    remove             Stop and disable archiver for a particular board. Does not delete any
                       data. Archiver will only stop after completing the current cycle.
    start              Start the archiver, API, and webserver
    start-read-only    Start in read only mode. Archivers will not run, only API and webserver
```
You can use `mitsuba help COMMAND` in order to get more detailed information on each command and the possible options:
```
$ mitsuba help add
mitsuba-add
Add a board to the archiver, or replace its settings.

USAGE:
    mitsuba add [OPTIONS] <NAME>

ARGS:
    <NAME>
            Board name (eg. 'po')

OPTIONS:
        --full-images <FULL_IMAGES>
            (Optional) If false, will only download thumbnails for this board. If true, thumbnails
            and full images. Default is false.

    -h, --help
            Print help information
```
This command will add a database entry for the specified board and its settings. An archiver will start for this board and any other boards you added with `add` as soon as the next archive cycle begins or mitsuba is (re)started.

As you can see there is only one option in terms of board specific settings.
- `full-images=true` will make the archiver download full images (and files) for that board. The default is `false`, meaning only thumbnails will be downloaded.

Note that any time you use `add` on a board that was already added before, it enables that board if it was disabled with `remove`, and *replaces* the configuration for that board with the values you specify, or the defaults. The previous settings are **ignored**. So if you had full image download enabled on /po/ previously with a wait time of 100, and then do `mitsuba add po`, the settings will be reset to the default of no full image download, and wait time of 10.

So let's add our first board, /po/ is a good example because it's the slowest board on 4chan most of the time:
```
mitsuba add po --full-images=true
Added /po/ Enabled: true, Full Images: true
```
We will get all images this way.
Let's confirm our changes with the `list` command which lists all boards added to mitsuba.
```
$ mitsuba list
/po/ Enabled: true, Full Images: true
1 boards found in database
```
- `Enabled` means the board is currently being actively archived.

Finally, we take a look at the `start` command:
```
$ mitsuba help start
mitsuba-start
Start the archiver, API, and webserver

USAGE:
    mitsuba start [OPTIONS]

OPTIONS:
        --archiver-only <ARCHIVER_ONLY>
            (Optional) If true, will only run the archiver and not the web ui or the web API. If
            false, run everything. Default is false.

        --burst <BURST>
            (Optional) Max burst of requests started at once. Default is 10.

    -h, --help
            Print help information

        --jitter-min <JITTER_MIN>
            (Optional) Minimum amount of jitter to add to rate-limiting to ensure requests don't
            start at the same time, in milliseconds. Default is 200ms

        --jitter-variance <JITTER_VARIANCE>
            (Optional) Variance for jitter, in milliseconds. Default is 800ms. Jitter will be a
            random value between min and min+variance.

        --max-time <MAX_TIME>
            (Optional) Maximum amount of time to spend retrying a failed image download, in seconds.
            Default is 600s (10m).

        --rpm <RPM>
            (Optional) Max requests per minute. Default is 60.
```
This command starts the archiver(s) for the boards we added, as well as the web UI to browse the archives and the JSON API.
There are various options, all are related to rate-limiting for the archiver, except one: `archiver-only`.

`--archiver-only=true` will start mitsuba as just an archiver. Posts will be fetched, images downloaded, but there is no web UI or API to actually see the archived content. This option is not particularly useful, but it will use **considerably** less RAM, going down to ~9-17MB.
You can run the web UI and API separately with `mitsuba start-read-only`. You can even run multiple instances of the web UI at the same time if you want, as it's read-only like the name implies.

`--rpm` is the main rate-limiter option. It decides how many requests per minute are performed by mitsuba against 4chan's API and image servers, globally.
All of the rate limiter settings are global for all boards and images, but note that images and API-calls (which are used to fetch threads) are counted **separately** in the rate limiting.

This means that if you set say, RPM to 60, mitsuba will perform 60 requests per minute (at most, on average it will be much less depending on how many threads get updated) against 4chan's API to fetch new threads it finds, **and** it will do 60 requests per minute to fetch images, for a global total of 120 requests per minute done at most across all boards and all images. This separation ensures that even if there's a large backlog of images to download, mitsuba can continue to fetch new posts at the same time, without the two interfering with each other.

We can now start our archiver with `mitsuba start`.
It will start its work of archiving the board, /po/.
After a while you can browse your archive by visiting http://127.0.0.1:8080/po/1

You can start a read-only instance of mitsuba, which will only serve the web UI and API but not archive anything, with `mitsuba start-read-only`.

You can separate the archiver and the public web UI/API by running the archiver by itself using `mitsuba start --archiver-only=true`, and then launch `mitsuba start-read-only` separately. This means you can stop or restart the archiver without any visible disruption to users who are just browsing your archive.
You can also run multiple instances of `mitsuba start-read-only` if you want, or run the full archiver and web UI with `mitsuba start` along with additional read-only instances.

## Web UI
The web UI is currently made to look and work exactly like 4chan's Yotsuba Blue theme, except it's the same on every board (even non-worksafe boards).
We also include 4chan's official default "inline extension" with all of its features and options (the inline extension is licensed as MIT).

The JS code has been slightly modified through trial and error to work correctly with Mitsuba. If you find an issue, file a bug report.

In other works, Mitsuba's UI looks and works (almost) exactly like 4chan's UI because it is exactly the same. This was simply the easiest short term solution, and it allows us to leverage the fact that our API is the same as 4chan's, which makes the inline extension work.
In the future, we want to customize the theme with different colors at least, to distinguish it, and add features that are suited to an archive.

The Web UI **also works with Javascript disabled**, just like 4chan's UI does.

Most of 4chan's features are correctly displayed in the Web UI. One limitation is that currently only real country flags are supported, and "troll flags" (eg. "Anarcho Capitalist" flag) are not visible. However, these are only enabled on one board (/pol/).

Also all capcode posts (Mod, Admin, Manager, etc) will be displayed the same as "Moderator" with no distinction between roles. 
These posts are extremely rare anyways (besides Moderator, which is displayed correctly), and I don't think there are even any up currently, and the distinction doesn't seem super important. This is only a GUI issue anyhow.
Might be fixed eventually.

The Flash board also has some features we haven't implemented but Flash is dead.
## API
Mitsuba features a read-only JSON API that is designed to be compatible with 4chan's [official API](https://github.com/4chan/4chan-API).
We will not fully document it here, since that would be redundant. You can read their documentation, because the URIs and data returned are mostly the same. There are a few (non-breaking) changes that are explained below.

First, currently, only the [Threads](https://github.com/4chan/4chan-API/blob/master/pages/Threads.md) and [Indexes](https://github.com/4chan/4chan-API/blob/master/pages/Indexes.md) endpoints have been implemented.

We've also added an endpoint to serve individual posts.

- Threads: `/[board]/thread/[op ID].json` Serves a full 4chan thread, in the same format the official API uses.
- Indices: `/[board]/[1-...].json` Serves the content of a board's index page. This is the default page you see when you visit a board, for example https://boards.4channel.org/po/ . On 4chan there are normally only 15 index pages, going for example from  `/po/1.json` to `/po/15.json`. On Mitsuba, since old threads are never deleted, there are as many pages as are needed to list all of the threads currently on the archive. Once there are no more threads, higher index numbers will return a 404 status code. This means you can easily scrape a mitsuba archive by fetching progressively higher indices until it 404s. Note that the order is the same as on 4chan, so it's not guaranteed to remain consistent. The order is based on which thread has had the most recent new post, not when the thread was first archived. Index pages don't contain full threads; they only show the OP and the last few replies to each thread.

In addition to these endpoints, we have implemented a `/[board]/post/[ID].json` endpoint that serves an individual post. Using this, you can fetch a post through its ID without needing to know the OP's.

There's also one extra endpoint that's entirely specific to Mitsuba: `/boards-status.json`, this returns the same data as the CLI's `list` command, but in JSON format.

### Differences with 4chan API
The main difference with 4chan's API is that every post also contains a `board` field with the name of the board it is in.

Moreover, in any situation where 4chan's API would omit a field, we instead include that field with a default value.

The only types present in 4chan's API are integers and strings, so for integers the value would be `0` and for strings it's an empty string.
This means that all JSON posts returned by our API are guaranteed to contain *all 37 fields* that a post could contain, no matter the board or type of post.
For example, on 4chan's side, the `sticky` field is only ever set on OP posts, and its value is `1` if it is set. On our side, for non-sticky posts, it would be present and set to `0`.

In practice this should not cause any issues with existing code targeting 4chan's API as long as you are checking the truthiness of each field rather than just its presence.

Two extra fields present on posts returned by our API are `file_sha256` and `thumbnail_sha256`.
These represent the SHA256 hashes of the attached file and thumbnail for each post.
Will be set to empty strings if not present or unavailable.

### Images
In addition to the API being compatible, we also support getting the images from the same paths 4chan uses.

For example, if an image on 4chan is served from https://i.4cdn.org/po/1546293948883.png, Mitsuba will serve it under `/po/1546293948883.png` and its corresponding thumbnail will be `/po/1546293948883s.jpg` just like on the original site.

This allows you to get an image from the archive *even if you only have the original link* (which might be dead now) and don't know the post or thread ID.

Note that this is intended to help you find lost images, but it's not the correct way to serve them to many users. **Visiting these links involves a database lookup every time** because we store images differently compared to how 4chan does it.

You can see the correct link for static images being used on the web UI. The URL is based on the image's SHA256 hash calculated by mitsuba, as opposed to the MD5 hash as supplied by 4chan, encoded in Base32.

For example, the link to a full image could be `/img/full/HA/T/HATHD6AY6NVOYH2JYEPI6ETKC2VFNUIVFHM4EEE5BXO4CQU6WDGA.png`, a thumbnail `/img/thumb/HA/T/HATHD6AY6NVOYH2JYEPI6ETKC2VFNUIVFHM4EEE5BXO4CQU6WDGA.jpg` where `HATHD6AY6NVOYH2JYEPI6ETKC2VFNUIVFHM4EEE5BXO4CQU6WDGA` is the full base32 sha256 hash, and the path consists in that plus a prefix with the first two characters of the hash, followed by the third character.

The `/img/` path serves all images directly from disk unless the S3 backend is enabled. Mitsuba looks in your `DATA_ROOT` folder, which is `data` by default, and serves the `images` folder within from this path (`/img/`). So you can find all the images in there.

## Proxies 
Mitsuba can be configured to use one or multiple proxies for requests to 4chan's API as well as the image fetching.
The load balancing system distributes the requests between them, allowing you to circumvent 4chan's rate limiting.
This is mainly intended for when it's strictly necessary, for example when 4chan is under DDoS attack which results in Cloudflare rate limiting clients to a degree that causes issues for archivers. This feature should not be used to abuse 4chan's API.

In order to add proxies, you need to set environment variables `PROXY_URL_{N}` to the URLs of the proxies you intend to use, where N is a number
starting from 0 for the first proxy, then 1 for the second, etc.
For example:
```
PROXY_URL_0=socks5://user:password@example.com:1337
PROXY_URL_1=socks5://user:password@10.0.0.2:1337
```
You have to start from 0 and not skip any numbers, or some or all of the proxies will not be detected.
You can set a weight for each proxy, this is an integer that determines how often it is used in relation to the others.
```
PROXY_WEIGHT_0=3
PROXY_WEIGHT_1=1
```
By default, even if you have proxies configured, Mitsuba **will** still use your machine's regular IP address and connection alongside the proxies you set, treating it as if it was a proxy with weight 1. Meaning that, if you set up two proxies, mitsuba will alternate between proxy 0, proxy 1 and your own IP for requests, based on the assigned weights.
This can be configured with `PROXY_ONLY`. If set to true, all requests will be routed through the proxies.
The weight of your own IP address as a "proxy" in load balancing can be set with `PROXY_WEIGHT_SELF`:
```
PROXY_ONLY=false
PROXY_WEIGHT_SELF=2
```
Note that the underlying HTTP library we use employs connection pooling. This means that the same connection to the server can be reused many times.
Whenever a connection is reused, the proxy will be the same as for the previous request that used the same connection. So, which proxy is used is only decided when a new connection is created, rather than whenever a request is made.
This means that the weights don't determine exactly how often the proxy is used to make requests, instead they determine how often they are used to open a new connection. Since connections can be reused any number of times, there is no guarantee that all proxies you configured will be used.

## Commands
Use `mitsuba help` to get a list of commands and their descriptions, `mitsuba help COMMAND` to see the options specific to each command.

### List
`mitsuba list`

Returns the list of boards currently known to the archiver. Includes "removed" boards that are not being archived, which are set as Enabled: False.

### Add
`mitsuba add BOARD [options]`

Adds a board to the database if not present, and sets it to enabled. Next time the archiver is started it will archive this board as well.
If the board was not present or disabled when this command was run, and Mitsuba's archiver is currently running, you need to restart it before the board actually starts being archived.

You can set some options for the board, which are stored in the database. `help add` to get more information on the options.
If you don't specify an option, it will be set to the default value. Even if the board was already in the database, and the setting was different, if you use `add` again, the setting will be reset to the default value unless you specify a different value.

### Remove
`mitsuba remove BOARD`

Does *not* remove a board from the database, does *not* delete any of its data, posts or images.

The only thing this does is **disable** a board's archiver job. That is, it stops being archived. The archiver for this board will complete its current cycle if it's in the middle of one, and then shut down. Other boards will not be affected.

This also does not affect the Web UI or the API. Data that was already archived for this board is still served like normal, just it won't be updated.

You can enable a board again by using `add`, however this doesn't apply until you restart mitsuba.

### Start
`mitsuba start`

Starts the archivers and the web UI and web API server. There are various settings you can check with `help start`, most of them are settings for the rate limiter.

The option `--archiver-only=true` will only start the archiver without the API or web UI. This is useful if you want to start the web UI/API separately using `start-read-only`, this setup allows you to restart or stop the archiver without causing any disruption to the public facing website.

### Start Read Only
`mitsuba start-read-only`

Starts the web UI and API, without the archivers. You can run as many instances of Mitsuba in this read-only mode as you wish.

## Administration
Mitsuba does not come with any admin UI besides the CLI commands above.

There are no commands to purge a board from the database or delete its images.

Images are stored in the same folders for all boards so you can't easily remove all the images just for one board, and some images might have been posted on multiple boards.

Thumbnails and full images **are** stored in different folders (`full` and `thumb`) so you can delete all full images (for all boards) by just deleting that entire folder if you want to.

However, if you really need to delete a particular image, you can just delete the file. The images on disk are never read, and whether an image has been downloaded or not is tracked in the database. So Mitsuba doesn't know that the image was removed from disk, and will never download it again, because it would be marked as already present.

It's safe to delete any of the images on disk, but of course they will return 404s if someone tries to access them.

We want to eventually have some convenient CLI tools for administration. Maybe a web UI in the future, but that's a bigger endeavour.

## Future

Some features that might be added:
- Search: this is by far the biggest missing feature. I wanted to have this in 1.0 but I didn't have the time. Ideally, we should have full text search for post content and titles, names and such, plus all the advanced search options foolfuuka has. There are multiple ways to go about this, right now I am considering two. The first option is to use Postgresql full text search. This is limited, but it actually does have all the features realistically needed for our use case, and it doesn't add any external dependencies. This would ensure search is always available. The second option is using `meilisearch` which is a rust full text search engine that is both lightweight and easy to set up, it works well out of the box. This would give us more capabilities, it is very flexible. But it's an external dependency that users would have to download and run separately like Postgresql itself. So I am hesitant to add it, and it would have to be optional, making search not always available. On the other hand, using something like elasticsearch would be a huge dependency that I'm not willing to take, and it requires a lot more configuration. Also it's the opposite of lightweight among web servers.
- `purge BOARD` command, to delete all archived data relative to a particular board. Would delete all posts, and all images that were only posted on that board, while preserving images also present on others. Would have option to only delete full images and preserve thumbnails and post data.
- `hide ID` command, to hide a particular post from being served by the API, or an entire thread if the ID is of a thread OP. Would simply mark the post as "hidden" on the database, without deleting, it, but it would no longer be publically visible. Mainly meant in cases where someone was doxxed and asked to have the info taken down. This would also have an option to delete the images associated with the post or thread that aren't present on other posts, while still keeping them marked as downloaded, so they would not be downloaded again.
- `check` command, to check the image store for corruption or missing images. This would first scan the database to see all images that are marked as downloaded, and then ensure that they exist on disk, hash the files to compare MD5s to make sure they are not corrupted. If an image is missing or corrupted, it would correct the database, and mark it as absent. Might also delete images that are on disk but aren't tracked in the database.

At the moment a full imageboard engine with posting and administration is considered out of scope, however if you are interested in working on that, you should make an issue to discuss it.