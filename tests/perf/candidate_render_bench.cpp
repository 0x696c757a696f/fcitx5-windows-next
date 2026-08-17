#include "candidate_model.h"

#include <Windows.h>
#include <d2d1.h>
#include <dwrite.h>
#include <wrl/client.h>

#include <chrono>
#include <cstdint>
#include <iostream>
#include <string>

namespace {

LRESULT CALLBACK windowProcedure(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
    return DefWindowProcW(window, message, wparam, lparam);
}

} // namespace

int main() {
    using Microsoft::WRL::ComPtr;
    using namespace fcitx::windows::candidate;
    const HINSTANCE instance = GetModuleHandleW(nullptr);
    WNDCLASSW windowClass{};
    windowClass.hInstance = instance;
    windowClass.lpszClassName = L"Fcitx5CandidateRenderBenchmark";
    windowClass.lpfnWndProc = windowProcedure;
    if (!RegisterClassW(&windowClass) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) return 1;
    HWND window = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                                  windowClass.lpszClassName, L"", WS_POPUP,
                                  0, 0, 720, 360, nullptr, nullptr, instance, nullptr);
    if (!window) return 1;

    ComPtr<ID2D1Factory> d2d;
    ComPtr<IDWriteFactory> dwrite;
    ComPtr<ID2D1HwndRenderTarget> target;
    ComPtr<IDWriteTextFormat> format;
    ComPtr<ID2D1SolidColorBrush> textBrush;
    if (FAILED(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,
                                 d2d.GetAddressOf())) ||
        FAILED(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                                   reinterpret_cast<IUnknown**>(dwrite.GetAddressOf()))) ||
        FAILED(d2d->CreateHwndRenderTarget(
            D2D1::RenderTargetProperties(),
            D2D1::HwndRenderTargetProperties(window, D2D1::SizeU(720, 360),
                                              D2D1_PRESENT_OPTIONS_IMMEDIATELY),
            &target)) ||
        FAILED(dwrite->CreateTextFormat(L"Segoe UI", nullptr, DWRITE_FONT_WEIGHT_NORMAL,
                                        DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                                        16.0F, L"zh-CN", &format)) ||
        FAILED(target->CreateSolidColorBrush(D2D1::ColorF(D2D1::ColorF::Black),
                                             &textBrush))) {
        DestroyWindow(window);
        return 1;
    }

    constexpr std::uint64_t kIterations = 1'000;
    CandidateModel model;
    const auto started = std::chrono::steady_clock::now();
    for (std::uint64_t revision = 1; revision <= kIterations; ++revision) {
        Snapshot snapshot{1, 1, 1, revision, "ni", {}, {}, {}, 0, 0, 9,
                          Visibility::composition};
        for (std::uint64_t index = 1; index <= 9; ++index) {
            snapshot.candidates.push_back(
                Item{index, std::to_string(index), "candidate", "annotation"});
        }
        if (model.apply(std::move(snapshot)) != ApplyResult::applied) return 1;
        target->BeginDraw();
        target->Clear(D2D1::ColorF(0.97F, 0.97F, 0.97F));
        float top = 8.0F;
        for (const auto& item : model.current()->candidates) {
            std::wstring line(item.label.begin(), item.label.end());
            line += L". candidate  annotation";
            target->DrawTextW(line.data(), static_cast<UINT32>(line.size()), format.Get(),
                              D2D1::RectF(12, top, 708, top + 32), textBrush.Get());
            top += 36.0F;
        }
        if (FAILED(target->EndDraw())) return 1;
    }
    const auto elapsed = std::chrono::steady_clock::now() - started;
    const double nanoseconds = static_cast<double>(
        std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count());
    std::cout << "candidate-model-to-paint-ns/op=" << nanoseconds / kIterations << '\n';
    DestroyWindow(window);
    return 0;
}
