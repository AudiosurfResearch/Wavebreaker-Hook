#include "D3D9Hook.hpp"
#include "DInputHook.hpp"
#include "WindowsMessageHook.hpp"
#include "utility.hpp"
#include "InterFont.hpp"

#include <imgui.h>
#include <imgui_freetype.h>
#include <imgui_impl_dx9.h>
#include <imgui_impl_win32.h>

#include <spdlog/spdlog.h>

IMGUI_IMPL_API LRESULT ImGui_ImplWin32_WndProcHandler(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

namespace wavebreaker
{
    namespace overlay
    {
        std::unique_ptr<D3D9Hook> m_d3d9Hook;
        std::unique_ptr<DInputHook> m_dinputHook;
        std::unique_ptr<WindowsMessageHook> m_wmHook;
        HWND m_wnd;

        bool m_initializedUI = false;
        bool m_isUIOpen = false;

        bool on_message(HWND wnd, UINT message, WPARAM wParam, LPARAM lParam)
        {
            if (m_isUIOpen)
            {
                if (ImGui_ImplWin32_WndProcHandler(wnd, message, wParam, lParam) != 0)
                {
                    // If the user is interacting with the UI we block the message from going to the game.
                    auto &io = ImGui::GetIO();

                    if (io.WantCaptureMouse || io.WantCaptureKeyboard || io.WantTextInput)
                    {
                        return false;
                    }
                }
            }

            return true;
        }

        void init_ui()
        {
            if (m_initializedUI)
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

            ImFont* font = io.Fonts->AddFontFromMemoryCompressedTTF(InterFont_compressed_data, InterFont_compressed_size, 16.0f);
            ImGuiFreeType::BuildFontAtlas(io.Fonts, 0);
            ImGui::StyleColorsDark();

            if (!ImGui_ImplWin32_Init(m_wnd))
            {
                spdlog::error("Failed to initialize ImGui (Win32).");
            }

            if (!ImGui_ImplDX9_Init(device))
            {
                spdlog::error("Failed to initialize ImGui (DX9).");
            }

            //
            // DInputHook.
            //
            spdlog::info("Hooking DInput...");

            m_dinputHook = std::make_unique<DInputHook>(m_wnd);

            if (!m_dinputHook->isValid())
            {
                spdlog::error("Failed to hook DInput.");
            }

            //
            // WindowsMessageHook.
            //
            spdlog::info("Hooking the windows message procedure...");

            m_wmHook = std::make_unique<WindowsMessageHook>(m_wnd);

            m_wmHook->onMessage = [](auto wnd, auto msg, auto wParam, auto lParam)
            {
                return on_message(wnd, msg, wParam, lParam);
            };

            if (!m_wmHook->isValid())
            {
                spdlog::error("Failed to hook windows message procedure.");
            }

            m_initializedUI = true;
        }

        void draw_ui()
        {
            ImGui::ShowDemoWindow();
        }

        void on_frame()
        {
            if (!m_initializedUI)
                init_ui();

            ImGui_ImplDX9_NewFrame();
            ImGui_ImplWin32_NewFrame();
            ImGui::NewFrame();

            if (utility::was_key_pressed(VK_INSERT))
            {
                m_isUIOpen = !m_isUIOpen;
            }

            if (utility::was_key_pressed(VK_END))
            {
                m_initializedUI = false;
            }

            if (m_isUIOpen)
            {
                // Block input if the user is interacting with the UI.
                auto &io = ImGui::GetIO();

                if (io.WantCaptureMouse || io.WantCaptureKeyboard || io.WantTextInput)
                {
                    m_dinputHook->ignoreInput();
                }
                else
                {
                    m_dinputHook->acknowledgeInput();
                }

                draw_ui();
            }
            else
            {
                // UI is closed so always pass input to the game.
                m_dinputHook->acknowledgeInput();
            }

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