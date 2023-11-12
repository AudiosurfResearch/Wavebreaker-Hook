#include <toml.hpp>
#include <spdlog/spdlog.h>

namespace wavebreaker
{
    namespace config
    {
        std::string server;
        bool verbose;
        bool forceInsecure;

        void init()
        {
            spdlog::info("Loading config");
            auto data = toml::parse("Wavebreaker-Client.toml");

            server = toml::find<std::string>(data, "server");
            verbose = toml::find<bool>(data, "verbose");
            forceInsecure = toml::find<bool>(data, "forceInsecure");
        }
    }
}