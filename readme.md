# Mitsuba
Mitsuba is a lightweight 4chan board archiver written in Rust. It continuously monitors a set of 4chan boards, fetches new posts, thumbnails, and optionally full images, and makes them available through an imageboard web UI as well as a read-only JSON API that is compatible with 4chan's [official API](https://github.com/4chan/4chan-API).

Mitsuba's main goal is to be very lightweight in terms of CPU and memory usage, and Rust helps accomplish this goal. Mitsuba is designed to be easy to deploy and doesn't currently have any runtime dependencies besides Postgresql and the related libraries.

The intended usage is self-hosting an archive on a low budget, however the Actix based web UI and API are quite performant, and should be capable of scaling to any amount of readers, with much lower resource consumption compared to competing frameworks in other languages, and without the possible latency spikes caused by garbage collection.

Mitsuba does not support "ghost posting" as it's not an imageboard engine. This could be supported in the future with some work (mostly on the front-end) but it requires actual administration tools and an accounts system, neither of which are actually present. What few options Mitsuba has are set through the CLI.

## Features
- Very quick and easy to set up
- Few dependencies, just Postgresql for now.
- Extremely lightweight, can run on a very small VPS.
- Fully integrated, archiving boards, serving them through a JSON API and Web UI all in one.
- Easy administration with a few CLI commands
- Configurable rate limiter
- Optional full image download setting per-board
- Web UI has a field that lets you jump to any post by typing its ID and selecting the board

There are some important features missing:
- No "ghost posting" or posting of any kind. Read only archive.
- No full text search, or any search really. Can only get to a post or thread from the ID. We want to have search eventually.
- No admin UI, CLI only (but there are few things to change anyways)
- No administration tools to delete individual posts or images (but you can safely delete an image file from the folder if necessary)
- No account system whatsoever (however this also makes it inherently secure)
- No tools to delete a board's posts or images if you don't want to have them anymore (This is a planned feature, for now you could just run one instance per board)

## Dependencies
You need to have Postgresql running on your system, the client libraries must be available on your path.
To install the libraries on Ubuntu:
```
sudo apt install libpq-dev libpq5
```
Your Postgresql server needs to be configured to accept at least 500 connections (`max_connections=500`) for mitsuba to work.
## Quick Setup (with little explanation)
```
export DATABASE_URI="postgres://user:password@127.0.0.1/mitsuba"
export RUST_LOG="info"
mitsuba add po
mitsuba start
```
After some threads have been archived, you can visit http://127.0.0.1:8080/po/1 to see your new archive for the /po/ board.

This will only get posts and thumbnails but not full images.

Use `mitsuba add po --full-images=true` to change that.

It's recommended to perform `mitsuba reset po` if you enable full images after you started the archiver with only thumbnails. This will not delete any data, but it makes it rescan the entire and get full images for posts that had already been archived. Otherwise you will only get full images on a thread once it gets a new post.

`mitsuba add BOARD` with a new board that is not being archived will not start an archiver for that board while mitsuba is already running. So if you want to add another board you have to restart mitsuba after `add`.
`mitsuba remove BOARD` *does* work while mitsuba is running. It doesn't delete any data, but it stops the archiver for that board. It will only stop after it has completed its current archival cycle. Restarting an archiver requires using `add` and then restarting mitsuba entirely.

## Setup
Mitsuba is designed to be easy and quick to set up.
Download a binary build for your system, or clone the repository and build your executable.
Mitsuba also requires some static files for its frontend, they should be included with the binary build and the source code as the "static" folder.
Just include them in the same directory mitsuba is run from. If you only need the API, they are not required.

Some options need to be passed as environment variables. Mitsuba uses `dotenv`, which means that instead of setting the environment variables, you can specify their values in a file called `.env` which must be in the directory you are running the mitsuba executable in. Mitsuba will read this file and apply the environment variables specified with their values.

You will find an `example.env` file in this repository. Copy it and rename it to just `.env`, then edit its configuration as needed.
A full explanation of the settings and their effect is included in this readme file in a dedicated section, but there are a couple of values that you need to be aware of right now:

- DATABASE_URI: you need to specify the connection URI for your Postgresql instance here. In the example file it's `postgres://user:password@127.0.0.1/mitsuba` .
   Replace with the correct username and password, as well as address, port and database name. The user needs to either be the owner of the specified database (in this case called `mitsuba`) if it already exists, or you need to create it yourself.

- DATA_ROOT: the directory in which to save the image files. This is an optional setting. If you leave it out entirely, Mitsuba will just create a "data" folder in the current working directory, and use that.

- RUST_LOG: the recommended setting for this is "info". This controls the mitsuba's log output. For now mitsuba uses `env_logger` which just prints the log information to standard output (the console). If this is not set, you will only see errors and warnings and no other feedback. 
  
The only required setting is DATABASE_URI but RUST_LOG="info" is also recommended. Just use the `example.env` file contents and change the database URI.

We will refer to the executable as `mitsuba` in this guide from now on, but on Windows® it is of course called `mitsuba.exe` .

Run `mitsuba help` to get a quick usage guide with a list of possible commands:

```
$ mitsuba help
High performance board archiver software. Add boards with `add`, then start with `start` command.
See `help add` and `help start`

USAGE:
    mitsuba.exe <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add                Add a board to the archiver, or replace its settings. (Requires restart
                       of mitsuba to apply changes)
    help               Prints this message or the help of the given subcommand(s)
    list               List all boards in the database and their current settings. Includes
                       stopped ('removed') boards
    remove             Stop archiver for a board. Does not delete any data, does not reset the
                       board. Archiver will only stop after completing the current cycle.
    reset              Reset a board's state. Does not delete any data or images. Next time the
                       archiver runs, it will fetch all currently active threads again from
                       scratch. Images will not be redownloaded unless they are missing. Run
                       this if you think the archiver missed some posts or images.
    start              Start the archiver, API, and webserver
    start-read-only    Start in read only mode. Archivers will not run, only API and webserver
```
You can use `mitsuba help COMMAND` in order to get more detailed information on each command and the possible options:
```
$ mitsuba help add
Add a board to the archiver, or replace its settings. (Requires restart of mitsuba to apply changes)

USAGE:
    mitsuba.exe add [OPTIONS] <name>

ARGS:
    <name>    Board name (eg. 'po')

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --full-images <full-images>    (Optional) If false, will only download thumbnails for this
                                       board. If true, thumbnails and full images. Default is false.
        --wait-time <wait-time>        (Optional) Seconds to wait after an update for this board is
                                       completed, before trying to perform a new update. Default is
                                       10s
```
This command will add a database entry for the specified board and its settings. Next time Mitsuba is run, an archiver will start for this board and any other boards you added with `add`.

As you can see there are two options for the board's settings. 
- `full-images=true` will make the archiver download full images (and files) for that board. The default is `false`, meaning only thumbnails will be downloaded.

 - `wait-time=10` specifies how long, in seconds, the archiver for this board will wait after it *finishes* fetching updates to get more updates. A longer time means it will get updates less often. It's recommended you set this to a higher value for slow boards that have relatively few posts per hour/minute. For very slow boards like /po/ even an hour could be safe. The tradeoff with this is that if you set it too high, a thread might get deleted and 404 before you capture some of the last posts. However most 4chan boards have 1-week archives after the threads die, so this is not really a concern.


 Wait time is a particularly important setting if you are archiving many boards with the same mitsuba instance. A slow board should get a higher wait time than a fast board with many posts per minute, otherwise if they are constantly updating at the same time, they can slow each other down. Mitsuba employs (configurable) rate-limiting for all requests to 4chan to respect their limits. This rate limiting is global, not per board. So if two boards are updated at the same time, it will take longer.

Note that any time you use `add` on a board that was already added before, it enables that board if it was disabled with `remove`, and *replaces* the configuration for that board with the values you specify, or the defaults. The previous settings are **ignored**. So if you had full image download enabled on /po/ previously with a wait time of 100, and then do `mitsuba add po`, the settings will be reset to the default of no full image download, and wait time of 10.

So let's add our first board, /po/ is a good example because it's the slowest board on 4chan most of the time:
```
mitsuba add po --full-images=true --wait-time=600
Added /po/ Enabled: true, Full Images: true, Wait Time: 600, Last Change: 0
If mitsuba is running, you need to restart it to apply these changes.
```
We will get all images, and we'll look for new thread updates 10 minutes after the previous update completes.
Let's confirm our changes with the `list` command which lists all boards added to mitsuba.
```
$ mitsuba list
/po/ Enabled: true, Full Images: true, Wait Time: 600, Last Change: 1616275933
1 boards found in database
```
- `Enabled` means the board is currently being actively archived.
- `Last change` is the timestamp of the most recent post mitsuba has archived on this board. On the next archive cycle, it will only fetch threads that have posts newer than this. Threads not updated since will be ignored. Initially the number is always 0 as we don't have any posts at all.
This value can be reset to 0 with `mitsuba reset BOARD`. This will not delete any data, but it will make the archiver rescan the entire board on the next cycle. All threads will be fetched again, but images won't be downloaded twice. However if you enabled full-images and full images are missing, they will be downloaded. So this is highly recommended if you add full-images to an existing board that was already being archived. Applying a reset doesn't require restart.

Finally, we take a look at the `start` command:
```
$ mitsuba help start
Start the archiver, API, and webserver

USAGE:
    mitsuba.exe start [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --archiver-only <archiver-only>        (Optional) If true, will only run the archiver and
                                               not the web ui or the web API. If false, run
                                               everything. Default is false.
        --burst <burst>                        (Optional) Max burst of requests started at once.
                                               Default is 10.
        --jitter-min <jitter-min>              (Optional) Minimum amount of jitter to add to rate-
                                               limiting to ensure requests don't start at the same
                                               time, in milliseconds. Default is 200ms
        --jitter-variance <jitter-variance>    (Optional) Variance for jitter, in milliseconds.
                                               Default is 800ms. Jitter will be a random value
                                               between min and min+variance.
        --max-time <max-time>                  (Optional) Maximum amount of time to spend retrying a
                                               failed image download, in seconds. Default is 60s
                                               (10m).
        --rpm <rpm>                            (Optional) Max requests per minute. Default is 60.
```
This command starts the archiver(s) for the boards we added, as well as the web UI to browse the archives and the JSON API.
There are various options, all are related to rate-limiting for the archiver, except one: `archiver-only`.

`--archiver-only=true` will start mitsuba as just an archiver. Posts will be fetched, images downloaded, but there is no web UI or API to actually see the archived content. This option is not particularly useful, but it will use **considerably** less RAM, going down to ~9-17MB.
You can run the web UI and API separately with `mitsuba start-read-only`. You can even run multiple instances of the web UI at the same time if you want, as it's read-only like the name implies.

`--rpm` is the main rate-limiter option. It decides how many requests per minute are performed by mitsuba against 4chan's API and image servers, globally.
All of the rate limiter settings are global for all boards and images, but note that images and API-calls (which are used to fetch threads) are counted **separately** in the rate limiting.

This means that if you set say, RPM to 60, mitsuba will perform 60 requests per minute (at most, on average it will be much less depending on how many threads get updated, and `wait-time`) against 4chan's API to fetch new threads it finds, **and** it will do 60 requests per minute to fetch images, for a global total of 120 requests per minute done at most across all boards and all images. This separation ensures that even if there's a large backlog of images to download, mitsuba can continue to fetch new posts at the same time, without the two interfering with each other.

We can now start our archiver with `mitsuba start`.
It will start its work of archiving the board, /po/.
After a while you can browse your archive by visiting http://127.0.0.1:8080/po/1

You can start a read-only instance of mitsuba, which will only serve the web UI and API but not archive anything, with `mitsuba start-read-only`.

You can separate the archiver and the public web UI/API by running the archiver by itself using `mitsuba start --archiver-only=true`, and then launch `mitsuba start-read-only` separately. This means you can stop or restart the archiver, add new boards to be archived, without any visible disruption to users who are just browsing your archive.
You can also run multiple instances of `mitsuba start-read-only` if you want, or run the full archiver and web UI with `mitsuba start` along with additional read-only instances.

## Web UI
The web UI is currently made to look and work exactly like 4chan's Yotsuba Blue theme, except it's the same on every board (even non-worksafe boards).
We also include 4chan's official default "inline extension" with all of its features and options (the inline extension is licensed as MIT).

The JS code has been slightly modified through trial and error to work correctly with Mitsuba. If you find an issue, file a bug report.

In other works, Mitsuba's UI looks and works (almost) exactly like 4chan's UI because it is exactly the same. This was simply the easiest short term solution, and it allows us to leverage the fact that our API is the same as 4chan's, which makes the inline extension work.
In the future, we want to customize the theme with different colors at least, to distinguish it, and add features that are suited to an archive.

The Web UI **also works with Javascript disabled**, just like 4chan's UI does.

Most of 4chan's features are correctly displayed in the Web UI. One limitation is that currently only real country flags are supported, and "troll flags" (eg. "Anarcho Capitalist" flag) are simply not displayed in the UI. However, these are only enabled on one board (/pol/).

Also all capcode posts (Mod, Admin, Manager, etc) will be displayed the same as "Moderator" with no distinction between roles. 
These posts are extremely rare anyways (besides, Moderator, which is displayed correctly), and I don't think there are even any up currently, and the distinction doesn't seem super important.
Might be fixed eventually.

The Flash board also has some features we haven't implemented but Flash is dead anyway.
### Note:
Currently the web UI is very inflexible when it comes to URLs. For example for a thread, you have to visit `/[board]/thread/[id]` , something like `/po/thread/570368/welcome-to-po` will get you a 404 because of the trailing `welcome-to-po`.

Index pages need to have the index number in the URL explicitly, so `/po/1` for example works, but `/po/` by itself returns 404.
This is WIP.
Also, there is no homepage `/` whatsoever.
## API
Mitsuba features a read-only JSON API that is designed to be compatible with 4chan's [official API](https://github.com/4chan/4chan-API).
Because of that, we will not fully document it here, since that would be redundant. You can read their documentation, because the URIs and data returned are mostly the same. There are a few (non-breaking) differences that are explained below (of course, besides the difference that this API is not served from 4chan's domains, but instead from your own server).

First, currently, only the [Threads](https://github.com/4chan/4chan-API/blob/master/pages/Threads.md) and [Indexes](https://github.com/4chan/4chan-API/blob/master/pages/Indexes.md) endpoints have been implemented.
In addition to that, we added an endpoint to serve individual posts.

- Threads: `/[board]/thread/[op ID].json` Serves a full 4chan thread, in the same format the official API uses.
- Indexes: `/[board]/[1-...].json` Serves the content of a board's index page. This is the default page you see when you visit a board, for example https://boards.4channel.org/po/ . On 4chan there are normally only 15 index pages, going for example from  `/po/1.json` to `/po/15.json`. On Mitsuba, since old threads are never deleted, there are as many pages as are needed to list all of the threads currently on the archive. Once there are no more threads, higher index numbers will return a 404 status code. This means you can easily scrape a mitsuba archive getting progressively higher indexes until it 404s. However note that the order is the same as on 4chan, so it's not guaranteed to remain consistent. The order is based on which thread has had the most recent new post, not when the thread was first archived. Also pages don't contain full threads, but only the OP and a few posts.

In addition to these endpoints, we have implemented a `/[board]/post/[ID].json` endpoint that just serves a single post by itself. Using this, you can get a post even if you only know its own ID and not the OP's.

There's also one extra endpoint that's entirely specific to Mitsuba: `/boards-status.json`, this returns the same data as the CLI's `list` command, but in JSON format.

### Differences with 4chan API
The main difference with 4chan's API is that every post also contains a `board` field with the name of the board it is in.

In addition to that, in every situation where 4chan's API would not return a field, we instead return that field with a default value.

The only types present in 4chan's API are integers and strings, so for integers the value would  `0` and for strings it's an empty string.
This means that all JSON posts returned by our API are guaranteed to contain *all 37 fields* that a post could contain, no matter the board or type of post.
For example, on 4chan's side, `sticky` field is only ever set on OP posts, and its value is `1` if it is set. On our side, if it wasn't set by 4chan, it would be present but set to `0`.

In practice this should not cause any issues with existing code targeting 4chan's API as long as you are checking the truthiness of each field rather than just its presence. Arguably, this should be easier to handle in most languages, since all fields are always guaranteed to be present.

### Images
In addition to the API being compatible, we also support getting the images from the same paths 4chan uses.

For example, if an image on 4chan is served from https://i.4cdn.org/po/1546293948883.png, Mitsuba will serve it under `/po/1546293948883.png` and its corresponding thumbnail will be `/po/1546293948883s.jpg` just like on the original site.

This allows you to get an image from the archive *even if you only have the original link* (which might be dead now) and don't know the post or thread ID.

Note that this is intended to help you find lost images, but it's not the correct way to serve them to many users. **Visiting these links involves a database lookup every time** because we store images differently compared to how 4chan does it.

You can see the correct link for static images being used on the web UI. The URL is based on the image's MD5 hash as supplied by 4chan, but encoded in Base32.

For example, for the previous image, the link to the full image would be `/img/full/XG/XGKR4ZPAOXQFKUPYY43ETQPPKQ.png` , the thumbnail is at `/img/thumb/XG/XGKR4ZPAOXQFKUPYY43ETQPPKQ.jpg` .

The `/img/` path serves all images directly from disk. Mitsuba looks in your `DATA_ROOT` folder, which is `data` by default, and serves the `images` folder within from this path (`/img/`). So you can find all the images in there.

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
If you don't specify an option, it will be set to the default value. Even if the board was already in the database, and the setting was different, if you use `add`, the setting will be reset to the default value unless you specify a different value.

### Remove
`mitsuba remove BOARD`

Does *not* remove a board from the database, does *not* delete any of its data, posts or images, and does *not* reset the board (see `reset` command for the latter).

What it *does* do is **disable** a board. That is, it stops being archived. The archiver for this board will complete its current cycle if it's in the middle of one, and then shut down. Other boards will not be affected.

This also does not affect the Web UI or the API. Data that was already archived for this board is still served like normal, just it won't be updated.

You can enable a board again by using `add`, however this doesn't apply until you restart mitsuba.

### Reset
`mitsuba reset BOARD`

Does *not* remove a board from the database, does *not* delete any of its data, posts or images, and does *not* stop a board from being archived (see `remove` command for the latter).

This command only sets the `Last Change` value for the board to 0, as it is when a board is first added.

This value is a timestamp, which corresponds to the most recent post on this board that we have archived. On each archive cycle, the archiver only fetches threads that have had changes (such as new posts) since this timestamp. This means that a thread that hasn't gotten new replies or other changes, will not be fetched again or checked in any way.

Resetting this value to 0 means that on the next run, the archiver will fetch all threads that are currently on the board.
It won't download an image a second time if it's already present. But for example if last time a full image was not fetched, and now full-images is set to true for this board, the full image will be fetched if not present. Therefore resetting a board after enabling full images is recommended. Otherwise you will only get the full images for threads once they change.

### Start
`mitsuba start`

Starts the archivers and the web UI and web API server. There are various settings you can check with `help start`, most of them are settings for the rate limiter.

The option `--archiver-only=true` will only start the archiver without the API or web UI. This is useful if you want to start the web UI/API separately using `start-read-only`, this setup allows you to restart or stop the archiver without causing any disruption to the public facing website.

### Start Read Only

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
- Object storage (S3 compatible) option to store images. This is not very difficult to implement by itself, but it requires handling more error cases, for example retrying when an upload to the object storage provider fails. Right now if an image fails to be saved to disk, it is dropped immediately and no retries are made. But with a remote storage provider more things can fail, and they might only fail temporarily.

At the moment a full imageboard engine with posting and administration is considered out of scope, however if you are interested in working on that, you should make an issue to discuss it.

