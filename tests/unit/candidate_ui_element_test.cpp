#include "candidate_ui_element.h"

#include <OleAuto.h>
#include <wrl/client.h>

#include <iostream>

int main() {
    using namespace fcitx::windows;
    Microsoft::WRL::ComPtr<tsf::CandidateUiElement> element;
    element.Attach(new tsf::CandidateUiElement());
    ipc::KeyResult result;
    result.candidates = {{1, L"1", L"\x4f60", L"n\x01d0"},
                         {2, L"2", L"\x5462", {}}};
    result.selectedCandidate = 1;
    result.candidateTotal = 2;
    result.candidatePage = 3;
    result.candidateVisibility = 1;
    element->update(nullptr, result);
    UINT count = 0;
    UINT selected = 0;
    BOOL shown = FALSE;
    DWORD flags = 0;
    BSTR text = nullptr;
    UINT currentPage = UINT_MAX;
    UINT pageCount = 0;
    UINT pageStart = UINT_MAX;
    if (FAILED(element->GetCount(&count)) || count != 2 ||
        FAILED(element->GetSelection(&selected)) || selected != 1 ||
        FAILED(element->IsShown(&shown)) || !shown ||
        FAILED(element->GetString(0, &text)) || !text ||
        FAILED(element->GetUpdatedFlags(&flags)) || flags == 0 ||
        FAILED(element->GetPageIndex(&pageStart, 1, &pageCount)) ||
        pageCount != 1 || pageStart != 0 ||
        FAILED(element->GetCurrentPage(&currentPage)) || currentPage != 0) {
        SysFreeString(text);
        std::cerr << "UILess candidate semantics failed\n";
        return 1;
    }
    const std::wstring first(text, SysStringLen(text));
    SysFreeString(text);
    text = nullptr;
    if (first.find(L"\x4f60") == std::wstring::npos) return 1;
    if (FAILED(element->Show(FALSE)) ||
        FAILED(element->IsShown(&shown)) || shown ||
        FAILED(element->GetCount(&count)) || count != 2 ||
        FAILED(element->GetSelection(&selected)) || selected != 1 ||
        FAILED(element->GetString(1, &text)) || !text) {
        SysFreeString(text);
        std::cerr << "UILess hidden state lost candidate semantics\n";
        return 1;
    }
    const std::wstring second(text, SysStringLen(text));
    SysFreeString(text);
    if (second.find(L"\x5462") == std::wstring::npos) return 1;
    return 0;
}
