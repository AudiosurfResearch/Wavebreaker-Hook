#include <spdlog/spdlog.h>
#include <safetyhook.hpp>
#include <D3D9Hook.hpp>

using namespace std;

namespace wavebreaker
{
    static D3D9Hook *g_d3d9Hook{nullptr};

    D3D9Hook::D3D9Hook()
        : onPresent{},
          onPreReset{},
          onPostReset{},
          m_device{},
          m_presentHook{},
          m_resetHook{}
    {
        if (g_d3d9Hook == nullptr)
        {
            if (hook())
            {
                spdlog::info("D3D9Hook hooked successfully.");
            }
            else
            {
                spdlog::error("D3D9Hook failed to hook.");
            }
        }
    }

    D3D9Hook::~D3D9Hook()
    {
        // Explicitly unhook the methods we hooked so we can reset g_d3d9Hook.
        m_presentHook.reset();
        m_resetHook.reset();

        g_d3d9Hook = nullptr;
    }

    bool D3D9Hook::hook()
    {
        spdlog::debug("Entering D3D9Hook::hook().");

        // Set hook object preemptively -- otherwise, the hook is written and is likely
        // to execute and crash before we verify success.
        g_d3d9Hook = this;

        // All we do here is create a IDirect3DDevice9 so that we can get the address
        // of the methods we want to hook from its vtable.
        using D3DCreate9Fn = IDirect3D9 *(WINAPI *)(UINT);

        auto d3d9 = GetModuleHandleA("d3d9.dll");
        auto d3dCreate9 = (D3DCreate9Fn)GetProcAddress(d3d9, "Direct3DCreate9");

        if (d3dCreate9 == nullptr)
        {
            spdlog::error("Couldn't find Direct3DCreate9.");
            return false;
        }

        spdlog::debug("Got Direct3DCreate9 {0:p}", (void *)d3dCreate9);

        auto d3d = d3dCreate9(D3D_SDK_VERSION);

        if (d3d == nullptr)
        {
            spdlog::error("Failed to create IDirect3D9.");
            return false;
        }

        spdlog::debug("Got IDirect3D9 {0:p}", (void *)d3d);

        D3DPRESENT_PARAMETERS pp{};

        ZeroMemory(&pp, sizeof(pp));

        pp.Windowed = 1;
        pp.SwapEffect = D3DSWAPEFFECT_FLIP;
        pp.BackBufferFormat = D3DFMT_A8R8G8B8;
        pp.BackBufferCount = 1;
        pp.hDeviceWindow = GetDesktopWindow();
        pp.PresentationInterval = D3DPRESENT_INTERVAL_IMMEDIATE;

        IDirect3DDevice9 *device{nullptr};

        if (FAILED(d3d->CreateDevice(
                D3DADAPTER_DEFAULT,
                D3DDEVTYPE_NULLREF,
                GetDesktopWindow(),
                D3DCREATE_HARDWARE_VERTEXPROCESSING | D3DCREATE_NOWINDOWCHANGES,
                &pp,
                &device)))
        {
            spdlog::error("Failed to create IDirect3DDevice9.");
            d3d->Release();
            return false;
        }

        spdlog::debug("Got IDirect3DDevice9 {0:p}", (void *)device);

        // Grab the addresses of the methods we want to hook.
        auto present = (*(uintptr_t **)device)[17];
        auto reset = (*(uintptr_t **)device)[16];

        spdlog::debug("Got IDirect3DDevice9::Present {0:p}", (void *)present);
        spdlog::debug("Got IDirect3DDevice9::Reset {0:p}", (void *)reset);

        device->Release();
        d3d->Release();

        // Hook them.
        m_presentHook = safetyhook::create_inline(present, (uintptr_t)&D3D9Hook::present);
        m_resetHook = safetyhook::create_inline(reset, (uintptr_t)&D3D9Hook::reset);

        if (m_presentHook && m_resetHook)
        {
            return true;
        }
        else
        {
            // If a problem occurred, reset the hook.
            m_presentHook.reset();
            m_resetHook.reset();
            g_d3d9Hook = nullptr;
            return false;
        }
    }

    HRESULT D3D9Hook::present(IDirect3DDevice9 *device, CONST RECT *src, CONST RECT *dest, HWND wnd, CONST RGNDATA *dirtyRgn)
    {
        auto d3d9 = g_d3d9Hook;

        d3d9->m_device = device;

        // Call our present callback.
        if (d3d9->onPresent)
        {
            d3d9->onPresent(*d3d9);
        }

        // Call the original present.
        return d3d9->m_presentHook.stdcall<HRESULT>(device, src, dest, wnd, dirtyRgn);
    }

    HRESULT D3D9Hook::reset(IDirect3DDevice9 *device, D3DPRESENT_PARAMETERS *presentParams)
    {
        auto d3d9 = g_d3d9Hook;

        d3d9->m_device = device;

        // Call our pre reset callback.
        if (d3d9->onPreReset)
        {
            d3d9->onPreReset(*d3d9);
        }

        // Call the original reset.
        auto result = d3d9->m_resetHook.stdcall<HRESULT>(device, presentParams);

        // Call our post reset callback.
        if (result == D3D_OK && d3d9->onPostReset)
        {
            d3d9->onPostReset(*d3d9);
        }

        return result;
    }
}