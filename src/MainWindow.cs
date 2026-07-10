using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Effects;
using System.Windows.Threading;

namespace LocalPass
{
    internal static class Program
    {
        [STAThread]
        private static void Main()
        {
            Application application = new Application();
            application.ShutdownMode = ShutdownMode.OnMainWindowClose;
            MainWindow window = new MainWindow();
            application.SessionEnding += window.OnSessionEnding;
            application.Run(window);
        }
    }

    internal sealed partial class MainWindow : Window
    {
        private readonly Border _shell;
        private readonly StackPanel _body;
        private readonly Slider _length;
        private readonly TextBlock _lengthValue;
        private readonly TextBox _count;
        private readonly ToggleButton _lowercase;
        private readonly ToggleButton _uppercase;
        private readonly ToggleButton _numbers;
        private readonly ToggleButton _symbols;
        private readonly ToggleButton _avoidAmbiguous;
        private Button _theme;
        private ToggleButton _pin;
        private readonly Button _generate;
        private readonly Button _clear;
        private readonly Grid _resultsHost;
        private readonly ScrollViewer _resultsScroller;
        private readonly StackPanel _resultsPanel;
        private readonly TextBlock _status;
        private readonly DispatcherTimer _clipboardTimer;
        private readonly DispatcherTimer _statusHideTimer;
        private readonly DispatcherTimer _rollTimer;
        private readonly Stopwatch _clipboardAge;
        private readonly List<PasswordRow> _rows;

        private SensitiveClipboard _clipboard;
        private bool _active;
        private bool _clearRequested;
        private bool _pendingClose;
        private bool _allowClose;
        private bool _rolledUp;
        private double _expandedBodyHeight;

        internal MainWindow()
        {
            Resources.MergedDictionaries.Add(Theme.Resources);
            Title = "LocalPass";
            Width = 360;
            SizeToContent = SizeToContent.Height;
            MaxHeight = Math.Max(300, SystemParameters.WorkArea.Height - 20);
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            WindowStyle = WindowStyle.None;
            ResizeMode = ResizeMode.NoResize;
            AllowsTransparency = !Theme.HighContrast;
            Background = Theme.HighContrast ? SystemColors.WindowBrush : Brushes.Transparent;
            if (Theme.HighContrast)
                Foreground = SystemColors.WindowTextBrush;
            else
                SetResourceReference(Control.ForegroundProperty, Theme.InkKey);
            FontFamily = Theme.UiFont;
            FontSize = 12.5;
            Topmost = true;
            ShowInTaskbar = true;
            UseLayoutRounding = true;
            SnapsToDevicePixels = true;
            Opacity = Theme.HighContrast ? 1 : 0.90;

            _rows = new List<PasswordRow>();
            _clipboardAge = new Stopwatch();

            _shell = new Border();
            _shell.Margin = new Thickness(10);
            _shell.Padding = new Thickness(16, 12, 16, 14);
            _shell.CornerRadius = new CornerRadius(18);
            if (Theme.HighContrast)
            {
                _shell.Background = SystemColors.WindowBrush;
                _shell.BorderBrush = SystemColors.WindowTextBrush;
            }
            else
            {
                _shell.SetResourceReference(Border.BackgroundProperty, Theme.ShellKey);
                _shell.SetResourceReference(Border.BorderBrushProperty, Theme.BubbleEdgeKey);
            }
            _shell.BorderThickness = new Thickness(1);
            if (!Theme.HighContrast)
            {
                _shell.Effect = new DropShadowEffect
                {
                    BlurRadius = 30,
                    ShadowDepth = 8,
                    Opacity = 0.45,
                    Color = Colors.Black
                };
            }

            StackPanel content = new StackPanel();
            _shell.Child = content;
            Content = _shell;
            content.Children.Add(BuildHeader());

            _body = new StackPanel();
            _body.ClipToBounds = true;
            content.Children.Add(_body);

            Grid settings = new Grid();
            settings.Margin = new Thickness(0, 12, 0, 0);
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(28) });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(10) });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(44) });

            TextBlock lengthLabel = Text("Length", 12, FontWeights.Normal, MutedBrush());
            lengthLabel.VerticalAlignment = VerticalAlignment.Center;
            settings.Children.Add(lengthLabel);

            _length = new Slider();
            _length.Minimum = 8;
            _length.Maximum = 64;
            _length.Value = 20;
            _length.TickFrequency = 1;
            _length.IsSnapToTickEnabled = true;
            _length.IsMoveToPointEnabled = true;
            _length.Height = 24;
            _length.Margin = new Thickness(10, 0, 7, 0);
            AutomationProperties.SetName(_length, "Password length");
            Apply(_length, Styles.SliderStyle());
            Grid.SetColumn(_length, 1);
            settings.Children.Add(_length);

            _lengthValue = Text("20", 12.5, FontWeights.SemiBold, InkBrush());
            _lengthValue.VerticalAlignment = VerticalAlignment.Center;
            _lengthValue.TextAlignment = TextAlignment.Right;
            Grid.SetColumn(_lengthValue, 2);
            settings.Children.Add(_lengthValue);

            TextBlock countLabel = Text("Count", 12, FontWeights.Normal, MutedBrush());
            countLabel.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(countLabel, 4);
            settings.Children.Add(countLabel);

            _count = new TextBox();
            _count.Text = "1";
            _count.Height = 30;
            _count.Margin = new Thickness(7, 0, 0, 0);
            _count.MaxLength = 2;
            _count.TextAlignment = TextAlignment.Center;
            _count.ToolTip = "Type 1 to 50; use Up and Down to adjust";
            AutomationProperties.SetName(_count, "Number of passwords, 1 to 50");
            Apply(_count, Styles.Input);
            Grid.SetColumn(_count, 5);
            settings.Children.Add(_count);
            _body.Children.Add(settings);

            StackPanel options = new StackPanel();
            options.Orientation = Orientation.Horizontal;
            options.Margin = new Thickness(0, 10, 0, 0);
            _lowercase = Option("a–z", 52, "Include lowercase letters");
            _uppercase = Option("A–Z", 52, "Include uppercase letters");
            _numbers = Option("0–9", 52, "Include numbers");
            _symbols = Option("!@#", 52, "Include symbols");
            _lowercase.IsChecked = true;
            _uppercase.IsChecked = true;
            _numbers.IsChecked = true;
            _symbols.IsChecked = true;
            options.Children.Add(_lowercase);
            options.Children.Add(_uppercase);
            options.Children.Add(_numbers);
            options.Children.Add(_symbols);
            _body.Children.Add(options);

            Grid actions = new Grid();
            actions.Margin = new Thickness(0, 10, 0, 0);
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

            _avoidAmbiguous = Option("No look-alikes", 132, "Exclude O, zero, I, l, and one");
            _avoidAmbiguous.IsChecked = true;
            _avoidAmbiguous.Margin = new Thickness(0);
            actions.Children.Add(_avoidAmbiguous);

            _clear = new Button();
            _clear.Content = "Clear";
            _clear.Width = 60;
            _clear.Height = 34;
            _clear.Margin = new Thickness(0, 0, 8, 0);
            _clear.Visibility = Visibility.Collapsed;
            AutomationProperties.SetName(_clear, "Clear generated passwords and owned clipboard content");
            Apply(_clear, Styles.GhostButton);
            Grid.SetColumn(_clear, 2);
            actions.Children.Add(_clear);

            _generate = new Button();
            _generate.Content = "Generate";
            _generate.Width = 98;
            _generate.Height = 34;
            AutomationProperties.SetName(_generate, "Generate passwords");
            Apply(_generate, Styles.PrimaryButton);
            Grid.SetColumn(_generate, 3);
            actions.Children.Add(_generate);
            _body.Children.Add(actions);

            _resultsHost = new Grid();
            _resultsHost.Height = 0;
            _resultsHost.Visibility = Visibility.Collapsed;
            _resultsHost.ClipToBounds = true;
            _resultsHost.Margin = new Thickness(0, 8, 0, 0);

            _resultsPanel = new StackPanel();
            _resultsScroller = new ScrollViewer();
            _resultsScroller.Content = _resultsPanel;
            _resultsScroller.VerticalScrollBarVisibility = ScrollBarVisibility.Auto;
            _resultsScroller.HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled;
            _resultsScroller.CanContentScroll = true;
            AutomationProperties.SetName(_resultsScroller, "Generated passwords");
            _resultsHost.Children.Add(_resultsScroller);
            _body.Children.Add(_resultsHost);

            _status = Text("", 12, FontWeights.Normal, MutedBrush());
            _status.Margin = new Thickness(1, 8, 0, 0);
            _status.TextTrimming = TextTrimming.CharacterEllipsis;
            _status.Visibility = Visibility.Collapsed;
            AutomationProperties.SetName(_status, "Status");
            _body.Children.Add(_status);

            _clipboardTimer = new DispatcherTimer(DispatcherPriority.Normal);
            _clipboardTimer.Interval = TimeSpan.FromMilliseconds(250);
            _statusHideTimer = new DispatcherTimer(DispatcherPriority.Background);
            _rollTimer = new DispatcherTimer(DispatcherPriority.Background);
            _rollTimer.Interval = TimeSpan.FromMilliseconds(450);

            WireEvents();
        }

        private Grid BuildHeader()
        {
            Grid header = new Grid();
            header.Height = 30;
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            header.MouseLeftButtonDown += DragHeader;

            TextBlock title = Text("LocalPass", 15, FontWeights.SemiBold, InkBrush());
            title.VerticalAlignment = VerticalAlignment.Center;
            header.Children.Add(title);

            _theme = new Button();
            _theme.Content = "\uE706";
            _theme.FontFamily = new FontFamily("Segoe MDL2 Assets");
            _theme.FontSize = 12;
            _theme.Width = 30;
            _theme.Height = 28;
            _theme.ToolTip = "Switch to light frost";
            AutomationProperties.SetName(_theme, "Switch light and dark frost theme");
            Apply(_theme, Styles.GhostButton);
            if (Theme.HighContrast)
                _theme.Visibility = Visibility.Collapsed;
            Grid.SetColumn(_theme, 1);
            header.Children.Add(_theme);

            _pin = new ToggleButton();
            _pin.Content = "\uE840";
            _pin.FontFamily = new FontFamily("Segoe MDL2 Assets");
            _pin.FontSize = 12;
            _pin.Width = 30;
            _pin.Height = 28;
            _pin.Margin = new Thickness(4, 0, 0, 0);
            _pin.IsChecked = true;
            _pin.ToolTip = "Keep on top";
            AutomationProperties.SetName(_pin, "Keep window always on top");
            Apply(_pin, Styles.Toggle);
            Grid.SetColumn(_pin, 2);
            header.Children.Add(_pin);

            Button close = new Button();
            close.Content = "\uE711";
            close.FontFamily = new FontFamily("Segoe MDL2 Assets");
            close.FontSize = 11;
            close.Width = 30;
            close.Height = 28;
            close.Margin = new Thickness(4, 0, 0, 0);
            close.ToolTip = "Close and clear";
            AutomationProperties.SetName(close, "Close and clear");
            Apply(close, Styles.GhostButton);
            close.Click += delegate { Close(); };
            Grid.SetColumn(close, 3);
            header.Children.Add(close);
            return header;
        }

        private ToggleButton Option(string text, double width, string accessibleName)
        {
            ToggleButton option = new ToggleButton();
            option.Content = text;
            option.Width = width;
            option.Height = 30;
            option.Margin = new Thickness(0, 0, 6, 0);
            AutomationProperties.SetName(option, accessibleName);
            Apply(option, Styles.Toggle);
            return option;
        }

        private static TextBlock Text(string value, double size, FontWeight weight, Brush brush)
        {
            TextBlock text = new TextBlock();
            text.Text = value;
            text.FontFamily = Theme.UiFont;
            text.FontSize = size;
            text.FontWeight = weight;
            if (!Theme.HighContrast && Object.ReferenceEquals(brush, Theme.Ink))
                text.SetResourceReference(TextBlock.ForegroundProperty, Theme.InkKey);
            else if (!Theme.HighContrast && Object.ReferenceEquals(brush, Theme.Muted))
                text.SetResourceReference(TextBlock.ForegroundProperty, Theme.MutedKey);
            else
                text.Foreground = brush;
            return text;
        }

        private static void Apply(Control control, Style style)
        {
            if (!Theme.HighContrast)
                control.Style = style;
        }

        private static Brush InkBrush()
        {
            return Theme.HighContrast ? SystemColors.WindowTextBrush : Theme.Ink;
        }

        private static Brush MutedBrush()
        {
            return Theme.HighContrast ? SystemColors.GrayTextBrush : Theme.Muted;
        }

        private sealed class PasswordRow
        {
            internal string Secret;
            internal TextBlock Display;
            internal Border Container;
            internal Button Copy;
            internal int Number;
        }
    }
}
