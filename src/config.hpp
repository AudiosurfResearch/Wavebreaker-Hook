#include <toml.hpp>

namespace wavebreaker
{
    namespace config
    {
        std::string server;
        bool verbose;

        void init()
        {
            spdlog::info("Loading config");
            auto data = toml::parse("Wavebreaker-Client.toml");

            server = toml::find<std::string>(data, "server");
            verbose = toml::find<bool>(data, "verbose");
        }
    }
}