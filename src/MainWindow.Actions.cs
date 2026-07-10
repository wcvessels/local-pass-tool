using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;

namespace LocalPass
{
    internal sealed partial class MainWindow
    {
        private void WireEvents()
        {
            SourceInitialized += delegate
            {
                WindowInteropHelper helper = new WindowInteropHelper(this);
                _clipboard = new SensitiveClipboard(delegate { return helper.Handle; });
            };

            Loaded += delegate
            {
                _active = true;
                _body.UpdateLayout();
                _expandedBodyHeight = _body.ActualHeight;
                Opacity = Theme.HighContrast ? 1 : 0.90;
            };
            Activated += delegate
            {
                _active = true;
                _rollTimer.Stop();
                SetSecretsVisible(true);
                Unroll();
                AnimateOpacityTo(0.90);
            };
            Deactivated += delegate
            {
                _active = false;
                SetSecretsVisible(false);
                if (IsMouseOver)
                {
                    _rollTimer.Stop();
                    Unroll();
                    AnimateOpacityTo(0.72);
                }
                else
                {
                    ScheduleRollUp();
                    AnimateOpacityTo(0.22);
                }
            };
            MouseEnter += delegate
            {
                _rollTimer.Stop();
                Unroll();
                if (!_active)
                    AnimateOpacityTo(0.72);
            };
            MouseLeave += delegate
            {
                if (!_active)
                {
                    ScheduleRollUp();
                    AnimateOpacityTo(0.22);
                }
            };
            _rollTimer.Tick += delegate
            {
                _rollTimer.Stop();
                if (!_active && !IsMouseOver)
                    RollUp();
            };

            _length.ValueChanged += delegate { _lengthValue.Text = ((int)_length.Value).ToString(); };
            _lowercase.Unchecked += KeepOneCharacterGroup;
            _uppercase.Unchecked += KeepOneCharacterGroup;
            _numbers.Unchecked += KeepOneCharacterGroup;
            _symbols.Unchecked += KeepOneCharacterGroup;
            _generate.Click += delegate { GeneratePasswords(); };
            _clear.Click += delegate { ClearAll(); };
            _theme.Click += delegate { ToggleTheme(); };
            _pin.Checked += delegate { Topmost = true; };
            _pin.Unchecked += delegate { Topmost = false; };

            _count.PreviewTextInput += CountPreviewTextInput;
            _count.PreviewKeyDown += CountKeyDown;
            _count.LostKeyboardFocus += delegate { ReadCount(); };
            DataObject.AddPastingHandler(_count, CountPaste);

            _clipboardTimer.Tick += ClipboardTick;
            _statusHideTimer.Tick += delegate
            {
                _statusHideTimer.Stop();
                HideStatus();
            };
            PreviewKeyDown += ShortcutKeyDown;
            Closing += WindowClosing;
        }

        private void GeneratePasswords()
        {
            bool oldClipboardReleased = RequestClipboardRelease();
            DropRows();

            try
            {
                int count = ReadCount();
                IList<string> passwords = PasswordGenerator.GenerateMany(
                    count,
                    (int)_length.Value,
                    _lowercase.IsChecked == true,
                    _uppercase.IsChecked == true,
                    _numbers.IsChecked == true,
                    _symbols.IsChecked == true,
                    _avoidAmbiguous.IsChecked == true);

                for (int i = 0; i < passwords.Count; i++)
                    AddPasswordRow(passwords[i], i + 1);

                SetSecretsVisible(_active);
                ShowResults(passwords.Count);

                if (oldClipboardReleased)
                    HideStatus();
                else
                    SetStatus("Waiting to release the previous clipboard value…", true, 0);

                if (_rows.Count > 0)
                {
                    Button firstCopy = _rows[0].Copy;
                    Dispatcher.BeginInvoke(
                        DispatcherPriority.Input,
                        new Action(delegate
                        {
                            if (firstCopy.IsVisible)
                                firstCopy.Focus();
                        }));
                }
            }
            catch (Exception exception)
            {
                CollapseResults();
                SetStatus(exception.Message, true, 2500);
            }
        }

        private void AddPasswordRow(string secret, int number)
        {
            PasswordRow row = new PasswordRow();
            row.Secret = secret;
            row.Number = number;

            Border container = new Border();
            container.Height = 40;
            container.Margin = new Thickness(0, 0, 0, 4);
            container.Padding = new Thickness(10, 4, 5, 4);
            container.CornerRadius = new CornerRadius(11);
            if (Theme.HighContrast)
            {
                container.Background = SystemColors.WindowBrush;
                container.BorderBrush = SystemColors.WindowTextBrush;
            }
            else
            {
                container.SetResourceReference(Border.BackgroundProperty, Theme.BubbleFillKey);
                container.SetResourceReference(Border.BorderBrushProperty, Theme.BubbleEdgeKey);
            }
            container.BorderThickness = new Thickness(1);
            AutomationProperties.SetName(container, "Password " + number);
            row.Container = container;

            Grid layout = new Grid();
            layout.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            layout.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            container.Child = layout;

            TextBlock display = Text("", 13, FontWeights.Normal, InkBrush());
            display.FontFamily = Theme.MonoFont;
            display.VerticalAlignment = VerticalAlignment.Center;
            display.TextTrimming = TextTrimming.CharacterEllipsis;
            display.Margin = new Thickness(0, 0, 7, 0);
            row.Display = display;
            layout.Children.Add(display);

            Button copy = new Button();
            copy.Content = "Copy";
            copy.MinWidth = 52;
            copy.Height = 30;
            AutomationProperties.SetName(copy, "Copy password " + number);
            Apply(copy, Styles.GhostButton);
            PasswordRow captured = row;
            copy.Click += delegate { CopyPassword(captured); };
            row.Copy = copy;
            Grid.SetColumn(copy, 1);
            layout.Children.Add(copy);

            _rows.Add(row);
            _resultsPanel.Children.Add(container);
        }

        private void CopyPassword(PasswordRow row)
        {
            if (_clipboard == null || !_clipboard.TryCopy(row.Secret))
            {
                SetStatus("Clipboard busy — copy failed", true, 2500);
                return;
            }

            _clearRequested = false;
            _clipboardAge.Restart();
            _clipboardTimer.Start();
            SetStatus("Password " + row.Number + " copied · clipboard releases in 30 s", false, 0);
        }

        private void ClearAll()
        {
            bool released = RequestClipboardRelease();
            CollapseResults();
            SetStatus(released ? "Cleared" : "Cleared · waiting for clipboard", !released, released ? 1500 : 0);
        }

        private bool RequestClipboardRelease()
        {
            _clipboardAge.Reset();
            if (_clipboard == null || _clipboard.TryClearOwned())
            {
                _clearRequested = false;
                _clipboardTimer.Stop();
                return true;
            }

            _clearRequested = true;
            _clipboardTimer.Start();
            return false;
        }

        private void ClipboardTick(object sender, EventArgs eventArgs)
        {
            if (_pendingClose)
            {
                if (_clipboard == null || _clipboard.TryClearOwnedOnce())
                {
                    _pendingClose = false;
                    _allowClose = true;
                    _clipboardTimer.Stop();
                    Dispatcher.BeginInvoke(DispatcherPriority.Send, new Action(Close));
                }
                return;
            }

            if (_clearRequested)
            {
                if (_clipboard == null || _clipboard.TryClearOwnedOnce())
                {
                    _clearRequested = false;
                    _clipboardTimer.Stop();
                    SetStatus("Clipboard released", false, 1500);
                }
                else
                {
                    SetStatus("Waiting for clipboard…", true, 0);
                }
                return;
            }

            int seconds = (int)Math.Ceiling(30 - _clipboardAge.Elapsed.TotalSeconds);
            if (seconds > 0)
            {
                SetStatus("Password copied · clipboard releases in " + seconds + " s", false, 0);
                return;
            }

            _clipboardAge.Stop();
            if (_clipboard == null || _clipboard.TryClearOwnedOnce())
            {
                _clipboardTimer.Stop();
                SetStatus("Clipboard released", false, 1500);
            }
            else
            {
                _clearRequested = true;
                SetStatus("Waiting for clipboard…", true, 0);
            }
        }

        private void ShowResults(int count)
        {
            double rowsHeight = Math.Min(3, count) * 44;
            _resultsScroller.Height = rowsHeight;
            double startHeight = _resultsHost.Visibility == Visibility.Visible
                ? _resultsHost.ActualHeight
                : 0;
            _resultsHost.Visibility = Visibility.Visible;
            _clear.Visibility = Visibility.Visible;
            AnimateHeight(_resultsHost, startHeight, rowsHeight, null);
        }

        private void CollapseResults()
        {
            double startHeight = _resultsHost.ActualHeight;
            SetSecretsVisible(false);
            DropRows();
            _clear.Visibility = Visibility.Collapsed;
            if (_resultsHost.Visibility != Visibility.Visible)
                return;

            AnimateHeight(_resultsHost, startHeight, 0, delegate
            {
                _resultsHost.Visibility = Visibility.Collapsed;
            });
        }

        private void DropRows()
        {
            foreach (PasswordRow row in _rows)
            {
                row.Display.Text = "";
                row.Secret = null;
            }
            _resultsPanel.Children.Clear();
            _rows.Clear();
        }

        private void SetSecretsVisible(bool visible)
        {
            foreach (PasswordRow row in _rows)
            {
                row.Display.Text = visible ? row.Secret : new string('•', row.Secret.Length);
                AutomationProperties.SetName(
                    row.Display,
                    "Password " + row.Number + (visible ? ", visible" : ", hidden"));
            }
        }

        private void ToggleTheme()
        {
            if (Theme.HighContrast)
                return;

            Theme.Apply(!Theme.IsLight);
            _length.Style = Styles.SliderStyle();
            _theme.Content = Theme.IsLight ? "\uE708" : "\uE706";
            _theme.ToolTip = Theme.IsLight ? "Switch to gunmetal frost" : "Switch to light frost";
            InvalidateVisual();
        }

        private void ScheduleRollUp()
        {
            if (Theme.HighContrast)
                return;
            _rollTimer.Stop();
            _rollTimer.Start();
        }

        private void RollUp()
        {
            if (_rolledUp || Theme.HighContrast)
                return;

            SetSecretsVisible(false);
            _expandedBodyHeight = Math.Max(1, _body.ActualHeight);
            _rolledUp = true;
            AnimateHeight(_body, _body.ActualHeight, 0, null);
        }

        private void Unroll()
        {
            _rollTimer.Stop();
            if (!_rolledUp || Theme.HighContrast)
                return;

            _body.BeginAnimation(FrameworkElement.HeightProperty, null);
            _body.Height = Double.NaN;
            _body.Measure(new Size(Math.Max(1, _body.ActualWidth), Double.PositiveInfinity));
            double target = Math.Max(_expandedBodyHeight, _body.DesiredSize.Height);
            _body.Height = 0;
            _rolledUp = false;
            AnimateHeight(_body, 0, target, delegate
            {
                _body.BeginAnimation(FrameworkElement.HeightProperty, null);
                _body.Height = Double.NaN;
            });
        }

        private void AnimateOpacityTo(double target)
        {
            if (Theme.HighContrast)
            {
                BeginAnimation(OpacityProperty, null);
                Opacity = 1;
                return;
            }

            if (!SystemParameters.ClientAreaAnimation)
            {
                BeginAnimation(OpacityProperty, null);
                Opacity = target;
                return;
            }

            DoubleAnimation animation = new DoubleAnimation();
            animation.From = Opacity;
            animation.To = target;
            animation.Duration = TimeSpan.FromMilliseconds(180);
            animation.EasingFunction = new QuarticEase { EasingMode = EasingMode.EaseOut };
            animation.FillBehavior = FillBehavior.Stop;
            animation.Completed += delegate
            {
                BeginAnimation(OpacityProperty, null);
                Opacity = target;
            };
            BeginAnimation(OpacityProperty, animation, HandoffBehavior.SnapshotAndReplace);
        }

        private static void AnimateHeight(
            FrameworkElement element,
            double from,
            double to,
            Action completed)
        {
            if (!SystemParameters.ClientAreaAnimation || Theme.HighContrast)
            {
                element.BeginAnimation(FrameworkElement.HeightProperty, null);
                element.Height = to;
                if (completed != null)
                    completed();
                return;
            }

            element.Height = Math.Max(0, from);
            DoubleAnimation animation = new DoubleAnimation();
            animation.From = Math.Max(0, from);
            animation.To = Math.Max(0, to);
            animation.Duration = TimeSpan.FromMilliseconds(190);
            animation.EasingFunction = new QuarticEase { EasingMode = EasingMode.EaseOut };
            animation.FillBehavior = FillBehavior.Stop;
            animation.Completed += delegate
            {
                element.BeginAnimation(FrameworkElement.HeightProperty, null);
                element.Height = to;
                if (completed != null)
                    completed();
            };
            element.BeginAnimation(
                FrameworkElement.HeightProperty,
                animation,
                HandoffBehavior.SnapshotAndReplace);
        }

        private void SetStatus(string text, bool warning, int autoHideMilliseconds)
        {
            _statusHideTimer.Stop();
            _status.Text = text;
            if (Theme.HighContrast)
                _status.Foreground = warning ? SystemColors.WindowTextBrush : SystemColors.GrayTextBrush;
            else
                _status.SetResourceReference(
                    TextBlock.ForegroundProperty,
                    warning ? Theme.WarningKey : Theme.MutedKey);
            _status.Visibility = Visibility.Visible;
            AutomationProperties.SetName(_status, "Status: " + text);

            if (autoHideMilliseconds > 0)
            {
                _statusHideTimer.Interval = TimeSpan.FromMilliseconds(autoHideMilliseconds);
                _statusHideTimer.Start();
            }
        }

        private void HideStatus()
        {
            _statusHideTimer.Stop();
            _status.Text = "";
            _status.Visibility = Visibility.Collapsed;
        }

        private void KeepOneCharacterGroup(object sender, RoutedEventArgs eventArgs)
        {
            if (_lowercase.IsChecked == true ||
                _uppercase.IsChecked == true ||
                _numbers.IsChecked == true ||
                _symbols.IsChecked == true)
                return;

            ToggleButton option = sender as ToggleButton;
            if (option != null)
                option.IsChecked = true;
            SetStatus("Keep at least one character group", true, 2000);
        }

        private int ReadCount()
        {
            int count;
            if (!Int32.TryParse(_count.Text, out count))
                count = 1;
            count = Math.Max(1, Math.Min(50, count));
            _count.Text = count.ToString();
            return count;
        }

        private void CountPreviewTextInput(object sender, TextCompositionEventArgs eventArgs)
        {
            eventArgs.Handled = !IsDigits(eventArgs.Text);
        }

        private void CountPaste(object sender, DataObjectPastingEventArgs eventArgs)
        {
            string text = eventArgs.DataObject.GetData(typeof(string)) as string;
            if (!IsDigits(text))
                eventArgs.CancelCommand();
        }

        private void CountKeyDown(object sender, KeyEventArgs eventArgs)
        {
            if (eventArgs.Key != Key.Up && eventArgs.Key != Key.Down)
                return;

            int count = ReadCount();
            count += eventArgs.Key == Key.Up ? 1 : -1;
            _count.Text = Math.Max(1, Math.Min(50, count)).ToString();
            _count.SelectAll();
            eventArgs.Handled = true;
        }

        private static bool IsDigits(string text)
        {
            if (String.IsNullOrEmpty(text))
                return false;
            foreach (char character in text)
            {
                if (!Char.IsDigit(character))
                    return false;
            }
            return true;
        }

        private void ShortcutKeyDown(object sender, KeyEventArgs eventArgs)
        {
            if ((Keyboard.Modifiers & ModifierKeys.Control) == 0)
                return;

            if (eventArgs.Key == Key.G)
            {
                GeneratePasswords();
                eventArgs.Handled = true;
            }
            else if (eventArgs.Key == Key.L)
            {
                ClearAll();
                eventArgs.Handled = true;
            }
        }

        private void DragHeader(object sender, MouseButtonEventArgs eventArgs)
        {
            DependencyObject source = eventArgs.OriginalSource as DependencyObject;
            while (source != null)
            {
                if (source is ButtonBase)
                    return;
                source = VisualTreeHelper.GetParent(source);
            }

            if (eventArgs.LeftButton == MouseButtonState.Pressed)
            {
                try { DragMove(); }
                catch (InvalidOperationException) { }
            }
        }

        private void WindowClosing(object sender, CancelEventArgs eventArgs)
        {
            SetSecretsVisible(false);
            DropRows();
            _rollTimer.Stop();

            if (_allowClose || _clipboard == null || _clipboard.TryClearOwned())
            {
                _clipboardTimer.Stop();
                _statusHideTimer.Stop();
                return;
            }

            eventArgs.Cancel = true;
            _pendingClose = true;
            _clearRequested = true;
            Hide();
            _clipboardTimer.Start();
        }

        internal void OnSessionEnding(object sender, SessionEndingCancelEventArgs eventArgs)
        {
            SetSecretsVisible(false);
            DropRows();
            _rollTimer.Stop();
            // Bounded best effort. Windows owns final clipboard disposal during
            // sign-out/shutdown; blocking the session would be worse behavior.
            if (_clipboard != null)
                _clipboard.TryClearOwned();
            _allowClose = true;
            _pendingClose = false;
            _clearRequested = false;
            _clipboardTimer.Stop();
            _statusHideTimer.Stop();
        }
    }
}
