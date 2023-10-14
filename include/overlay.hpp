#include "D3D9Hook.hpp"

#include <imgui.h>
#include <imgui_freetype.h>
#include <imgui_impl_dx9.h>
#include <imgui_impl_win32.h>

#include <spdlog/spdlog.h>

namespace wavebreaker
{
    namespace overlay
    {
        bool initializedUI = false;

        void init_ui() {

        }

        void on_frame() {
            if (!initializedUI) init_ui();
        }

        void init()
        {
            auto m_d3d9Hook = std::make_unique<wavebreaker::D3D9Hook>();

            m_d3d9Hook->onPresent = [](auto &)
            { on_frame(); };
            m_d3d9Hook->onPreReset = [](auto &)
            { ImGui_ImplDX9_InvalidateDeviceObjects(); };
            m_d3d9Hook->onPostReset = [](auto &)
            { ImGui_ImplDX9_CreateDeviceObjects(); };

            if (!m_d3d9Hook->isValid())
            {
                spdlog::error("Failed to hook D3D9.");
            }
        }
    }
}