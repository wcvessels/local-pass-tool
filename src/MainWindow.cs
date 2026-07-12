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
using System.Windows.Media.Effects;
using System.Windows.Threading;
using System.Threading;

namespace LocalPass
{
    internal static class Program
    {
        [STAThread]
        private static void Main()
        {
            bool firstInstance;
            using (Mutex instance = new Mutex(true, "Local\\LocalPass.5A4D9128-387F-4DAF-B86E-557927CE4898", out firstInstance))
            {
                if (!firstInstance)
                    return;

                Application application = new Application();
                application.ShutdownMode = ShutdownMode.OnMainWindowClose;
                MainWindow window = new MainWindow();
                application.SessionEnding += window.OnSessionEnding;
                application.Run(window);
            }
        }
    }

    internal sealed partial class MainWindow : Window
    {
        private readonly Border _shell;
        private readonly StackPanel _body;
        private readonly Grid _header;
        private StackPanel _headerButtons;
        private TextBlock _collapsedHint;
        private readonly Slider _length;
        private readonly TextBox _lengthValue;
        private readonly TextBox _count;
        private readonly ToggleButton _lowercase;
        private readonly ToggleButton _uppercase;
        private readonly ToggleButton _numbers;
        private readonly ToggleButton _symbols;
        private readonly ToggleButton _avoidAmbiguous;
        private Button _infoButton;
        private ToggleButton _mask;
        private Button _theme;
        private ToggleButton _pin;
        private Button _close;
        private readonly Button _generate;
        private readonly Button _clear;
        private readonly Grid _resultsHost;
        private readonly ScrollViewer _resultsScroller;
        private readonly StackPanel _resultsPanel;
        private readonly TextBlock _status;
        private readonly Popup _infoPopup;
        private readonly TextBlock _clipboardSecondsText;
        private readonly Button _clipMinus;
        private readonly Button _clipPlus;
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
        private int _clipboardSeconds = 30;

        internal MainWindow()
        {
            Resources.MergedDictionaries.Add(Theme.Resources);
            Title = "LocalPass";
            Width = 420;
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
            FontSize = 12;
            Topmost = true;
            ShowInTaskbar = true;
            UseLayoutRounding = true;
            SnapsToDevicePixels = true;

            _rows = new List<PasswordRow>();
            _clipboardAge = new Stopwatch();

            _shell = new Border();
            _shell.Margin = new Thickness(10);
            _shell.Padding = new Thickness(16, 12, 16, 14);
            _shell.CornerRadius = new CornerRadius(12);
            _shell.BorderThickness = new Thickness(1);
            if (Theme.HighContrast)
            {
                _shell.Background = SystemColors.WindowBrush;
                _shell.BorderBrush = SystemColors.WindowTextBrush;
            }
            else
            {
                _shell.SetResourceReference(Border.BackgroundProperty, Theme.ShellKey);
                _shell.SetResourceReference(Border.BorderBrushProperty, Theme.ShellEdgeKey);
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

            _header = BuildHeader();
            content.Children.Add(_header);

            _body = new StackPanel();
            _body.ClipToBounds = true;
            content.Children.Add(_body);

            Border topDivider = Divider();
            topDivider.Margin = new Thickness(0, 11, 0, 11);
            _body.Children.Add(topDivider);

            Grid settings = new Grid();
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(34) });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            settings.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(34) });

            TextBlock lengthLabel = Label("LENGTH");
            lengthLabel.VerticalAlignment = VerticalAlignment.Center;
            settings.Children.Add(lengthLabel);

            _lengthValue = NumberInput("20", "Password length, 4 to 64");
            _lengthValue.Margin = new Thickness(10, 0, 0, 0);
            Grid.SetColumn(_lengthValue, 1);
            settings.Children.Add(_lengthValue);

            _length = new Slider();
            _length.Minimum = 4;
            _length.Maximum = 64;
            _length.Value = 20;
            _length.TickFrequency = 1;
            _length.IsSnapToTickEnabled = true;
            _length.IsMoveToPointEnabled = true;
            _length.Height = 26;
            _length.Margin = new Thickness(10, 0, 10, 0);
            AutomationProperties.SetName(_length, "Password length slider");
            Apply(_length, Styles.SliderStyle());
            Grid.SetColumn(_length, 2);
            settings.Children.Add(_length);

            TextBlock countLabel = Label("COUNT");
            countLabel.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(countLabel, 3);
            settings.Children.Add(countLabel);

            _count = NumberInput("3", "Number of passwords, 1 to 99");
            _count.Margin = new Thickness(10, 0, 0, 0);
            Grid.SetColumn(_count, 4);
            settings.Children.Add(_count);
            _body.Children.Add(settings);

            Grid options = new Grid();
            options.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            options.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            options.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            options.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            options.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1.3, GridUnitType.Star) });
            _lowercase = Segment("a–z", "Include lowercase letters", 0);
            _uppercase = Segment("A–Z", "Include uppercase letters", 1);
            _numbers = Segment("0–9", "Include numbers", 2);
            _symbols = Segment("!@#", "Include symbols", 3);
            _avoidAmbiguous = Segment("NO 0O1l", "Exclude look-alike characters", 4);
            _lowercase.IsChecked = true;
            _uppercase.IsChecked = true;
            _numbers.IsChecked = true;
            _symbols.IsChecked = true;
            _avoidAmbiguous.IsChecked = true;
            options.Children.Add(_lowercase);
            options.Children.Add(_uppercase);
            options.Children.Add(_numbers);
            options.Children.Add(_symbols);
            options.Children.Add(_avoidAmbiguous);
            Border segmentBar = new Border();
            segmentBar.Margin = new Thickness(0, 11, 0, 0);
            segmentBar.Height = 30;
            segmentBar.CornerRadius = new CornerRadius(6);
            segmentBar.BorderThickness = new Thickness(1);
            segmentBar.SetResourceReference(Border.BorderBrushProperty, Theme.BubbleEdgeKey);
            segmentBar.Child = options;
            _body.Children.Add(segmentBar);

            Grid actions = new Grid();
            actions.Margin = new Thickness(0, 11, 0, 0);
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(8) });
            actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(76) });

            _generate = new Button();
            _generate.Content = "G E N E R A T E";
            _generate.Height = 30;
            _generate.FontSize = 11;
            AutomationProperties.SetName(_generate, "Generate passwords");
            Apply(_generate, Styles.PrimaryButton);
            actions.Children.Add(_generate);

            _clear = new Button();
            _clear.Content = "C L E A R";
            _clear.Height = 30;
            _clear.FontSize = 11;
            _clear.IsEnabled = false;
            AutomationProperties.SetName(_clear, "Clear generated passwords and owned clipboard content");
            Apply(_clear, Styles.GhostButton);
            Grid.SetColumn(_clear, 2);
            actions.Children.Add(_clear);
            _body.Children.Add(actions);

            _resultsHost = new Grid();
            _resultsHost.Visibility = Visibility.Collapsed;
            _resultsHost.Margin = new Thickness(0, 11, 0, 0);
            _resultsHost.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            _resultsHost.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            _resultsHost.Children.Add(Divider());

            _resultsPanel = new StackPanel();
            _resultsScroller = new ScrollViewer();
            _resultsScroller.Content = _resultsPanel;
            _resultsScroller.Margin = new Thickness(0, 10, -12, 0);
            _resultsScroller.Resources[typeof(ScrollBar)] = Styles.ThinScrollBar;
            _resultsScroller.MaxHeight = 350;
            _resultsScroller.VerticalScrollBarVisibility = ScrollBarVisibility.Auto;
            _resultsScroller.HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled;
            _resultsScroller.CanContentScroll = true;
            AutomationProperties.SetName(_resultsScroller, "Generated passwords");
            Grid.SetRow(_resultsScroller, 1);
            _resultsHost.Children.Add(_resultsScroller);
            _body.Children.Add(_resultsHost);

            _status = Text("", 10.5, FontWeights.Normal, MutedBrush());
            _status.FontFamily = Theme.MonoFont;
            _status.Margin = new Thickness(0, 8, 0, 0);
            _status.TextWrapping = TextWrapping.Wrap;
            _status.Visibility = Visibility.Collapsed;
            AutomationProperties.SetName(_status, "Status");
            _body.Children.Add(_status);

            _infoPopup = new Popup();
            _infoPopup.PlacementTarget = _shell;
            _infoPopup.Placement = PlacementMode.Right;
            _infoPopup.HorizontalOffset = 14;
            _infoPopup.VerticalOffset = 0;
            _infoPopup.AllowsTransparency = true;
            _infoPopup.StaysOpen = false;
            _infoPopup.Child = BuildInfoPane(out _clipboardSecondsText, out _clipMinus, out _clipPlus);

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
            header.Height = 24;
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            header.MouseLeftButtonDown += DragHeader;

            TextBlock title = Text("L O C A L P A S S", 12, FontWeights.SemiBold, InkBrush());
            title.VerticalAlignment = VerticalAlignment.Center;
            header.Children.Add(title);

            _headerButtons = new StackPanel();
            _headerButtons.Orientation = Orientation.Horizontal;
            Grid.SetColumn(_headerButtons, 1);
            header.Children.Add(_headerButtons);

            _infoButton = IconButton("?", "About LocalPass");
            _headerButtons.Children.Add(_infoButton);

            _mask = new ToggleButton();
            _mask.Content = "✱";
            _mask.ToolTip = "Mask passwords";
            _mask.Width = 24;
            _mask.Height = 24;
            _mask.FontSize = 11;
            _mask.Padding = new Thickness(0);
            AutomationProperties.SetName(_mask, "Mask passwords");
            Apply(_mask, Styles.IconToggle);
            _headerButtons.Children.Add(_mask);

            _pin = new ToggleButton();
            _pin.Content = "◉";
            _pin.ToolTip = "Pin on top";
            _pin.Width = 24;
            _pin.Height = 24;
            _pin.FontSize = 11;
            _pin.Padding = new Thickness(0);
            _pin.IsChecked = true;
            AutomationProperties.SetName(_pin, "Keep window always on top");
            Apply(_pin, Styles.IconToggle);
            _headerButtons.Children.Add(_pin);

            _theme = IconButton("◐", "Switch theme");
            if (Theme.HighContrast)
                _theme.Visibility = Visibility.Collapsed;
            _headerButtons.Children.Add(_theme);
            _close = IconButton("\u2715", "Close and clear");
            _headerButtons.Children.Add(_close);

            _collapsedHint = Label("CLICK TO EXPAND");
            _collapsedHint.VerticalAlignment = VerticalAlignment.Center;
            _collapsedHint.Visibility = Visibility.Collapsed;
            Grid.SetColumn(_collapsedHint, 1);
            header.Children.Add(_collapsedHint);
            return header;
        }

        private Border BuildInfoPane(out TextBlock secondsText, out Button minus, out Button plus)
        {
            Border pane = new Border();
            pane.Width = 320;
            pane.Padding = new Thickness(16, 12, 16, 14);
            pane.CornerRadius = new CornerRadius(12);
            pane.BorderThickness = new Thickness(1);
            pane.SetResourceReference(Border.BackgroundProperty, Theme.InfoShellKey);
            pane.SetResourceReference(Border.BorderBrushProperty, Theme.ShellEdgeKey);
            pane.Effect = new DropShadowEffect { BlurRadius = 40, ShadowDepth = 16, Opacity = 0.45, Color = Colors.Black };

            StackPanel content = new StackPanel();
            pane.Child = content;

            Grid heading = new Grid();
            heading.Height = 20;
            heading.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            heading.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            TextBlock title = Label("ABOUT LOCALPASS");
            title.VerticalAlignment = VerticalAlignment.Center;
            heading.Children.Add(title);
            Button close = IconButton("✕", "Close About");
            close.Width = 20;
            close.Height = 20;
            close.Click += delegate { _infoPopup.IsOpen = false; };
            Grid.SetColumn(close, 1);
            heading.Children.Add(close);
            content.Children.Add(heading);

            Border divider = Divider();
            divider.Margin = new Thickness(0, 10, 0, 10);
            content.Children.Add(divider);

            TextBlock privacy = Text("Passwords are generated on this device. Nothing is stored, logged, or sent.", 11.5, FontWeights.Normal, InkBrush());
            privacy.SetResourceReference(TextBlock.ForegroundProperty, Theme.SecondaryTextKey);
            privacy.TextWrapping = TextWrapping.Wrap;
            privacy.LineHeight = 17.5;
            content.Children.Add(privacy);

            StackPanel legend = new StackPanel();
            legend.Margin = new Thickness(0, 10, 0, 4);
            legend.Children.Add(InfoRow("LENGTH · COUNT", "size · batch"));
            legend.Children.Add(InfoRow("a–z … !@#", "character sets"));
            legend.Children.Add(InfoRow("NO 0O1l", "removes look-alikes"));
            legend.Children.Add(InfoRow("✱", "mask results"));
            legend.Children.Add(InfoRow("◉", "always on top"));
            content.Children.Add(legend);

            Border lowerDivider = Divider();
            lowerDivider.Margin = new Thickness(0, 0, 0, 10);
            content.Children.Add(lowerDivider);

            Grid timer = new Grid();
            timer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            timer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(38) });
            timer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(8) });
            timer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(22) });
            timer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(4) });
            timer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(22) });
            TextBlock timerLabel = Text("Clipboard clear timer", 11, FontWeights.SemiBold, InkBrush());
            timerLabel.VerticalAlignment = VerticalAlignment.Center;
            timer.Children.Add(timerLabel);
            secondsText = Text("30 S", 12, FontWeights.Normal, InkBrush());
            secondsText.FontFamily = Theme.MonoFont;
            secondsText.TextAlignment = TextAlignment.Center;
            secondsText.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(secondsText, 1);
            timer.Children.Add(secondsText);
            minus = IconButton("−", "Reduce clipboard clear timer");
            minus.Width = 22;
            minus.Height = 22;
            Apply(minus, Styles.GhostButton);
            Grid.SetColumn(minus, 3);
            timer.Children.Add(minus);
            plus = IconButton("+", "Increase clipboard clear timer");
            plus.Width = 22;
            plus.Height = 22;
            Apply(plus, Styles.GhostButton);
            Grid.SetColumn(plus, 5);
            timer.Children.Add(plus);
            content.Children.Add(timer);
            return pane;
        }

        private Grid InfoRow(string key, string description)
        {
            Grid row = new Grid();
            row.Margin = new Thickness(0, 0, 0, 6);
            row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(108) });
            row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            TextBlock keyText = Text(key, 11, FontWeights.SemiBold, InkBrush());
            keyText.FontFamily = Theme.MonoFont;
            row.Children.Add(keyText);
            TextBlock descriptionText = Text(description, 11, FontWeights.Normal, MutedBrush());
            Grid.SetColumn(descriptionText, 1);
            row.Children.Add(descriptionText);
            return row;
        }
        private ToggleButton Segment(string text, string accessibleName, int column)
        {
            ToggleButton option = new ToggleButton();
            option.Content = text;
            option.Height = 30;
            option.Margin = new Thickness(0);
            option.BorderThickness = new Thickness(column == 0 ? 0 : 1, 0, 0, 0);
            option.FontFamily = Theme.MonoFont;
            option.FontSize = 11.5;
            option.Padding = new Thickness(0);
            AutomationProperties.SetName(option, accessibleName);
            Apply(option, Styles.SegmentToggle);
            Grid.SetColumn(option, column);
            return option;
        }

        private Button IconButton(string text, string accessibleName)
        {
            Button button = new Button();
            button.Content = text;
            button.Width = 24;
            button.Height = 24;
            button.FontSize = 11;
            button.Padding = new Thickness(0);
            button.ToolTip = accessibleName;
            AutomationProperties.SetName(button, accessibleName);
            Apply(button, Styles.IconButton);
            return button;
        }

        private TextBox NumberInput(string value, string accessibleName)
        {
            TextBox input = new TextBox();
            input.Text = value;
            input.Width = 34;
            input.Height = 26;
            input.MaxLength = 2;
            input.FontFamily = Theme.MonoFont;
            input.FontSize = 12;
            input.TextAlignment = TextAlignment.Center;
            AutomationProperties.SetName(input, accessibleName);
            Apply(input, Styles.Input);
            return input;
        }

        private static Border Divider()
        {
            Border divider = new Border();
            divider.Height = 1;
            divider.SetResourceReference(Border.BackgroundProperty, Theme.DividerKey);
            return divider;
        }

        private static TextBlock Label(string value)
        {
            TextBlock label = Text(value, 10, FontWeights.SemiBold, MutedBrush());
            return label;
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
            internal bool Revealed;
        }
    }
}