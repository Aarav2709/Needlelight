using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;
using Needlelight.Models;
using Needlelight.ViewModels;

namespace Needlelight.Views.Pages;

public partial class InfoView : UserControl
{
    public InfoView()
    {
        InitializeComponent();
    }

    private void OnSelectHollowKnight(object? sender, RoutedEventArgs e)
    {
        _ = MainWindowViewModel.Instance?.SwitchGameAsync(GameProfiles.HollowKnightKey);
    }

    private void OnSelectSilksong(object? sender, RoutedEventArgs e)
    {
        _ = MainWindowViewModel.Instance?.SwitchGameAsync(GameProfiles.SilksongKey);
    }

    private void InitializeComponent()
    {
        AvaloniaXamlLoader.Load(this);
    }
}
