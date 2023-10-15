#include <D3D9Hook.hpp>
#include <DInputHook.hpp>

#include <imgui.h>
#include <imgui_freetype.h>
#include <imgui_impl_dx9.h>
#include <imgui_impl_win32.h>

#include <spdlog/spdlog.h>

namespace wavebreaker
{
    namespace overlay
    {
        std::unique_ptr<D3D9Hook> m_d3d9Hook;
        std::unique_ptr<DInputHook> m_dinputHook;
        HWND m_wnd;

        bool initializedUI = false;

        void init_ui()
        {
            if (initializedUI)
                return;

            spdlog::info("Initializing overlay");

            // Grab the HWND from the device's creation parameters.
            spdlog::debug("Getting window from D3D9 device...");

            auto device = m_d3d9Hook->getDevice();
            D3DDEVICE_CREATION_PARAMETERS creationParams{};

            device->GetCreationParameters(&creationParams);

            m_wnd = creationParams.hFocusWindow;

            //
            // ImGui.
            //
            spdlog::info("Initializing ImGui...");

            IMGUI_CHECKVERSION();
            ImGui::CreateContext();

            auto &io = ImGui::GetIO();

            // io.Fonts->AddFontFromMemoryCompressedTTF(g_font_compressed_data, g_font_compressed_size, 16.0f);
            // ImGuiFreeType::BuildFontAtlas(io.Fonts, 0);

            if (!ImGui_ImplWin32_Init(m_wnd))
            {
                spdlog::error("Failed to initialize ImGui (Win32).");
            }

            if (!ImGui_ImplDX9_Init(device))
            {
                spdlog::error("Failed to initialize ImGui (DX9).");
            }

            ImGui::StyleColorsDark();

            //
            // DInputHook.
            //
            spdlog::info("Hooking DInput...");

            m_dinputHook = std::make_unique<DInputHook>(m_wnd);

            if (!m_dinputHook->isValid())
            {
                spdlog::error("Failed to hook DInput.");
            }

            initializedUI = true;
        }

        void on_frame()
        {
            if (!initializedUI)
                init_ui();

            ImGui_ImplDX9_NewFrame();
            ImGui_ImplWin32_NewFrame();
            ImGui::NewFrame();

            ImGui::ShowDemoWindow();

            ImGui::EndFrame();
            ImGui::Render();
            ImGui_ImplDX9_RenderDrawData(ImGui::GetDrawData());
        }

        void init()
        {
            m_d3d9Hook = std::make_unique<D3D9Hook>();

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