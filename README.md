> [!IMPORTANT]
> If you are an end-user, please use [the installer](https://wavebreaker.arcadian.garden/installguide) to set this up, unless you have a good reason not to.

# Wavebreaker Client
This enables Audiosurf to connect to [the public testing instance of Wavebreaker](https://wavebreaker.arcadian.garden/).
It *should* be able to connect to the official server as well, but that is *not* its primary intended use and keeping compatibility with the original server is not explicitly a goal of this project.

## Features
- Forces HTTPS for every request.
- If present, the [MusicBrainz ID](https://musicbrainz.org/doc/MusicBrainz_Identifier) of the [recording](https://musicbrainz.org/doc/Recording) is sent to the server, if it's present in a song file's metadata.
- Can optionally force HTTP (without the S) to aid in custom server development.

## Manual install
Copy the client's DLL file to the game's ``channels`` folder, which is inside the ``engine`` folder.
Then, create your own ``Wavebreaker.toml`` file inside the ``engine`` folder itself (see below).

## Config file
The config file is named ``Wavebreaker.toml`` and can be found in the ``engine`` folder of the game's files.

### Example
This is an example config that contains the recommended values.

```toml
[main]
server = "wavebreaker.arcadian.garden" # Server to connect to
force_insecure = false # Forces HTTP when set to true. DON'T DO THIS! (Unless you need it for server development)
```