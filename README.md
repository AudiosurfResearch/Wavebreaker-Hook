**If you're an end-user and don't have an actual reason to install this manually, please use [the installer!](https://github.com/AudiosurfResearch/Wavebreaker-Installer)**

# Wavebreaker-Hook
DLL file that enables an Audiosurf game client to connect to [Wavebreaker](https://github.com/AudiosurfResearch/Wavebreaker) (or any other Audiosurf 1 server implementation).
This eliminates the need for any changes to the HOSTS file, which brings numerous advantages, like being easier to set up or allowing users to still be able to access the official site, if needed.

# Install
1. Find the game directory using Steam (Right-click Audiosurf in Library > Manage > Browse game files)
2. Enter ``engine`` folder
3. Enter ``channels`` folder
4. Paste DLL file
5. Launch game through steam

# How?
Through a special type of magic known as *function hooking.* To facilitate this, we use cursey's [SafetyHook](https://github.com/cursey/safetyhook).
