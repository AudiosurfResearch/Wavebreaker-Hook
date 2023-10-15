#define INITGUID

#include <spdlog/spdlog.h>
#include <DInputHook.hpp>

using namespace std;

namespace wavebreaker
{
    static DInputHook *g_dinputHook{nullptr};

    DInputHook::DInputHook(HWND wnd)
        : m_wnd{wnd}, m_getDeviceDataHook{}, m_deviceObjectData{}, m_isIgnoringInput{false}
    {
        if (g_dinputHook == nullptr)
        {
            if (hook())
            {
                spdlog::info("DInputHook hooked successfully.");
                g_dinputHook = this;
            }
            else
            {
                spdlog::error("DInputHook failed to hook.");
            }
        }
    }

    DInputHook::~DInputHook()
    {
        // Explicitly unhook the methods we hooked so we can reset g_dinputHook.
        m_getDeviceDataHook.reset();

        g_dinputHook = nullptr;
    }

    bool DInputHook::hook()
    {
        spdlog::debug("Entering DInputHook::hook().");

        // All we do here is create an IDirectInputDevice so that we can get the
        // addresses of the methods we want to hook from its vtable.
        using DirectInput8CreateFn = HRESULT(WINAPI *)(HINSTANCE, DWORD, REFIID, LPVOID *, LPUNKNOWN);

        auto dinput8 = GetModuleHandleA("dinput8.dll");
        auto dinput8Create = (DirectInput8CreateFn)GetProcAddress(dinput8, "DirectInput8Create");

        if (dinput8Create == nullptr)
        {
            spdlog::error("Failed to find DirectInput8Create.");
            return false;
        }

        spdlog::debug("Got DirectInput8Create {0:p}", (void *)dinput8Create);

        auto instance = (HINSTANCE)GetModuleHandle(nullptr);
        IDirectInput *dinput{nullptr};

        if (FAILED(dinput8Create(instance, DIRECTINPUT_VERSION, IID_IDirectInput8W, (LPVOID *)&dinput, nullptr)))
        {
            spdlog::error("Failed to create IDirectInput.");
            return false;
        }

        spdlog::debug("Got IDirectInput {0:p}", (void *)dinput);

        IDirectInputDevice *device{nullptr};

        if (FAILED(dinput->CreateDevice(GUID_SysKeyboard, &device, nullptr)))
        {
            spdlog::error("Failed to create IDirectInputDevice.");
            dinput->Release();
            return false;
        }

        spdlog::debug("Got IDirectInputDevice {0:p}", (void *)device);

        // Get the addresses of the methods we want to hook.
        auto getDeviceData = (*(uintptr_t **)device)[10];

        spdlog::debug("Got IDirectInputDevice::GetDeviceData {0:p}", (void *)getDeviceData);

        device->Release();
        dinput->Release();

        // Hook them.
        m_getDeviceDataHook = safetyhook::create_inline(getDeviceData, (uintptr_t)&DInputHook::getDeviceData);

        // because bool operator.
        return m_getDeviceDataHook && true;
    }

    HRESULT DInputHook::getDeviceData(
        IDirectInputDevice *device, DWORD size, LPDIDEVICEOBJECTDATA data, LPDWORD numElements, DWORD flags)
    {
        auto dinput = g_dinputHook;
        auto originalGetDeviceData = (decltype(DInputHook::getDeviceData) *)dinput->m_getDeviceDataHook.trampoline().address();

        // If we are ignoring input then we call the original to remove buffered
        // input events from the devices queue without modifying the out parameters.
        if (dinput->m_isIgnoringInput)
        {
            device->Unacquire();
            device->SetCooperativeLevel(dinput->m_wnd, DISCL_FOREGROUND | DISCL_NONEXCLUSIVE);
            device->Acquire();

            if (*numElements == -1 || data == nullptr) // detect buffer flush
            {
                return originalGetDeviceData(device, size, data, numElements, flags);
            }
            else
            {
                dinput->m_deviceObjectData.resize(*numElements);

                originalGetDeviceData(device, size, dinput->m_deviceObjectData.data(), numElements, flags);
            }

            *numElements = 0;

            return DI_OK;
        }

        auto result = originalGetDeviceData(device, size, data, numElements, flags);

        if (flags == 0)
        {
            // Call callbacks for key presses and releases.
            // Note: DIDEVICEOBJECTDATA can have different sizes based on what version of DInput was used, so we go through
            // them in a way that the size of the struct doesn't matter.
            struct DeviceObjectData
            {
                DWORD key;
                DWORD data;
            };

            auto start = (uintptr_t)data;
            auto end = start + size * *numElements;

            for (auto i = start; i < end; i += size)
            {
                auto obj_data = (DeviceObjectData *)i;

                // According to the documentation for DIDEVICEOBJECTDATA:
                // For button input, only the low byte of dwData is significant. The high bit of the low byte is set if the
                // button was pressed; it is clear if the button was released.
                if (obj_data->data & (1 << 7))
                {
                    if (dinput->onKeyDown)
                    {
                        dinput->onKeyDown(*dinput, obj_data->key);
                    }
                }
                else
                {
                    if (dinput->onKeyUp)
                    {
                        dinput->onKeyUp(*dinput, obj_data->key);
                    }
                }
            }
        }

        return result;
    }
}