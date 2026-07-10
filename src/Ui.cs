using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Data;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Effects;

namespace LocalPass
{
    internal static class Theme
    {
        internal static readonly bool HighContrast = SystemParameters.HighContrast;
        internal static readonly FontFamily UiFont = new FontFamily("Segoe UI");
        internal static readonly FontFamily MonoFont = new FontFamily("Cascadia Mono");

        internal const string InkKey = "LocalPass.Ink";
        internal const string MutedKey = "LocalPass.Muted";
        internal const string FocusKey = "LocalPass.Focus";
        internal const string PrimaryTextKey = "LocalPass.PrimaryText";
        internal const string WarningKey = "LocalPass.Warning";
        internal const string BubbleEdgeKey = "LocalPass.BubbleEdge";
        internal const string HighlightKey = "LocalPass.Highlight";
        internal const string TrackKey = "LocalPass.Track";
        internal const string ShellKey = "LocalPass.Shell";
        internal const string BubbleFillKey = "LocalPass.BubbleFill";
        internal const string BubbleHoverKey = "LocalPass.BubbleHover";
        internal const string BubblePressedKey = "LocalPass.BubblePressed";
        internal const string SelectedFillKey = "LocalPass.SelectedFill";
        internal const string PrimaryFillKey = "LocalPass.PrimaryFill";
        internal const string PrimaryHoverKey = "LocalPass.PrimaryHover";
        internal const string PrimaryPressedKey = "LocalPass.PrimaryPressed";

        // Replacing resources remains safe after WPF seals styles.
        internal static readonly ResourceDictionary Resources = new ResourceDictionary();
        internal static readonly Brush Transparent = Brushes.Transparent;

        internal static SolidColorBrush Ink { get { return SolidAt(InkKey); } }
        internal static SolidColorBrush Muted { get { return SolidAt(MutedKey); } }
        internal static SolidColorBrush Focus { get { return SolidAt(FocusKey); } }
        internal static SolidColorBrush PrimaryText { get { return SolidAt(PrimaryTextKey); } }
        internal static SolidColorBrush Warning { get { return SolidAt(WarningKey); } }
        internal static SolidColorBrush BubbleEdge { get { return SolidAt(BubbleEdgeKey); } }
        internal static SolidColorBrush Highlight { get { return SolidAt(HighlightKey); } }
        internal static SolidColorBrush Track { get { return SolidAt(TrackKey); } }
        internal static LinearGradientBrush Shell { get { return GradientAt(ShellKey); } }
        internal static LinearGradientBrush BubbleFill { get { return GradientAt(BubbleFillKey); } }
        internal static LinearGradientBrush BubbleHover { get { return GradientAt(BubbleHoverKey); } }
        internal static LinearGradientBrush BubblePressed { get { return GradientAt(BubblePressedKey); } }
        internal static LinearGradientBrush SelectedFill { get { return GradientAt(SelectedFillKey); } }
        internal static LinearGradientBrush PrimaryFill { get { return GradientAt(PrimaryFillKey); } }
        internal static LinearGradientBrush PrimaryHover { get { return GradientAt(PrimaryHoverKey); } }
        internal static LinearGradientBrush PrimaryPressed { get { return GradientAt(PrimaryPressedKey); } }

        internal static bool IsLight { get; private set; }

        static Theme()
        {
            Apply(false);
        }

        internal static void Apply(bool light)
        {
            IsLight = light;
            if (light)
            {
                Resources[InkKey] = Solid(255, 31, 34, 38);
                Resources[MutedKey] = Solid(255, 79, 86, 94);
                Resources[FocusKey] = Solid(255, 31, 34, 38);
                Resources[PrimaryTextKey] = Solid(255, 247, 248, 249);
                Resources[WarningKey] = Solid(255, 132, 83, 24);
                Resources[BubbleEdgeKey] = Solid(105, 78, 86, 95);
                Resources[HighlightKey] = Solid(205, 255, 255, 255);
                Resources[TrackKey] = Solid(150, 83, 91, 99);
                Resources[ShellKey] = Gradient(Color.FromArgb(230, 255, 255, 255), Color.FromArgb(242, 230, 234, 238));
                Resources[BubbleFillKey] = Gradient(Color.FromArgb(205, 255, 255, 255), Color.FromArgb(120, 216, 222, 227));
                Resources[BubbleHoverKey] = Gradient(Color.FromArgb(242, 255, 255, 255), Color.FromArgb(165, 215, 221, 226));
                Resources[BubblePressedKey] = Gradient(Color.FromArgb(170, 245, 247, 249), Color.FromArgb(120, 195, 202, 208));
                Resources[SelectedFillKey] = Gradient(Color.FromArgb(225, 255, 255, 255), Color.FromArgb(170, 196, 203, 209));
                Resources[PrimaryFillKey] = Gradient(Color.FromArgb(245, 91, 97, 105), Color.FromArgb(248, 43, 47, 53));
                Resources[PrimaryHoverKey] = Gradient(Color.FromArgb(250, 106, 113, 121), Color.FromArgb(250, 49, 54, 61));
                Resources[PrimaryPressedKey] = Gradient(Color.FromArgb(250, 68, 73, 80), Color.FromArgb(252, 33, 36, 41));
            }
            else
            {
                Resources[InkKey] = Solid(255, 244, 246, 248);
                Resources[MutedKey] = Solid(255, 178, 184, 190);
                Resources[FocusKey] = Solid(255, 248, 249, 250);
                Resources[PrimaryTextKey] = Solid(255, 248, 249, 250);
                Resources[WarningKey] = Solid(255, 229, 171, 102);
                Resources[BubbleEdgeKey] = Solid(92, 255, 255, 255);
                Resources[HighlightKey] = Solid(115, 255, 255, 255);
                Resources[TrackKey] = Solid(145, 184, 190, 196);
                Resources[ShellKey] = Gradient(Color.FromArgb(225, 46, 51, 57), Color.FromArgb(239, 20, 23, 27));
                Resources[BubbleFillKey] = Gradient(Color.FromArgb(78, 255, 255, 255), Color.FromArgb(24, 255, 255, 255));
                Resources[BubbleHoverKey] = Gradient(Color.FromArgb(112, 255, 255, 255), Color.FromArgb(43, 255, 255, 255));
                Resources[BubblePressedKey] = Gradient(Color.FromArgb(52, 255, 255, 255), Color.FromArgb(17, 255, 255, 255));
                Resources[SelectedFillKey] = Gradient(Color.FromArgb(125, 255, 255, 255), Color.FromArgb(48, 255, 255, 255));
                Resources[PrimaryFillKey] = Gradient(Color.FromArgb(235, 96, 103, 112), Color.FromArgb(245, 48, 53, 60));
                Resources[PrimaryHoverKey] = Gradient(Color.FromArgb(240, 114, 122, 132), Color.FromArgb(248, 56, 62, 70));
                Resources[PrimaryPressedKey] = Gradient(Color.FromArgb(240, 75, 81, 89), Color.FromArgb(250, 38, 42, 48));
            }
        }

        internal static object Dynamic(string key)
        {
            return new DynamicResourceExtension(key);
        }

        internal static Brush GlassBackground()
        {
            return HighContrast ? (Brush)SystemColors.WindowBrush : (Brush)Shell;
        }

        private static SolidColorBrush Solid(byte alpha, byte red, byte green, byte blue)
        {
            return new SolidColorBrush(Color.FromArgb(alpha, red, green, blue));
        }

        private static LinearGradientBrush Gradient(Color top, Color bottom)
        {
            LinearGradientBrush gradient = new LinearGradientBrush();
            gradient.StartPoint = new Point(0, 0);
            gradient.EndPoint = new Point(0, 1);
            gradient.GradientStops.Add(new GradientStop(top, 0));
            gradient.GradientStops.Add(new GradientStop(bottom, 1));
            return gradient;
        }

        private static SolidColorBrush SolidAt(string key)
        {
            return (SolidColorBrush)Resources[key];
        }

        private static LinearGradientBrush GradientAt(string key)
        {
            return (LinearGradientBrush)Resources[key];
        }
    }

    internal static class Styles
    {
        internal static readonly Style PrimaryButton = CreatePrimaryButton();
        internal static readonly Style GhostButton = CreateGhostButton();
        internal static readonly Style Toggle = CreateToggle();
        internal static readonly Style Input = CreateInput();

        internal static Style SliderStyle()
        {
            string focus = Theme.Focus.Color.ToString();
            string ink = Theme.Ink.Color.ToString();
            string track = Theme.Track.Color.ToString();
            string xaml =
                "<Style xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' " +
                "xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' TargetType='{x:Type Slider}'>" +
                "<Setter Property='Cursor' Value='Hand'/>" +
                "<Setter Property='Template'><Setter.Value>" +
                "<ControlTemplate TargetType='{x:Type Slider}'>" +
                "<Grid Height='24' Background='Transparent'>" +
                "<Border x:Name='Focus' Margin='0,1' CornerRadius='7' BorderThickness='2' BorderBrush='Transparent'/>" +
                "<Track x:Name='PART_Track' Margin='6,0' VerticalAlignment='Center' " +
                "Minimum='{TemplateBinding Minimum}' Maximum='{TemplateBinding Maximum}' " +
                "Value='{TemplateBinding Value}' Orientation='{TemplateBinding Orientation}' " +
                "IsDirectionReversed='{TemplateBinding IsDirectionReversed}'>" +
                "<Track.DecreaseRepeatButton>" +
                "<RepeatButton Command='{x:Static Slider.DecreaseLarge}' Focusable='False'>" +
                "<RepeatButton.Template><ControlTemplate TargetType='{x:Type RepeatButton}'>" +
                "<Border Height='4' CornerRadius='2' Background='" + focus + "'/>" +
                "</ControlTemplate></RepeatButton.Template></RepeatButton>" +
                "</Track.DecreaseRepeatButton>" +
                "<Track.IncreaseRepeatButton>" +
                "<RepeatButton Command='{x:Static Slider.IncreaseLarge}' Focusable='False'>" +
                "<RepeatButton.Template><ControlTemplate TargetType='{x:Type RepeatButton}'>" +
                "<Border Height='4' CornerRadius='2' Background='" + track + "'/>" +
                "</ControlTemplate></RepeatButton.Template></RepeatButton>" +
                "</Track.IncreaseRepeatButton>" +
                "<Track.Thumb><Thumb Width='14' Height='14' Focusable='False'>" +
                "<Thumb.Template><ControlTemplate TargetType='{x:Type Thumb}'>" +
                "<Border x:Name='Dot' CornerRadius='7' Background='" + ink + "' " +
                "BorderBrush='" + focus + "' BorderThickness='2'>" +
                "<Border.Effect><DropShadowEffect BlurRadius='5' ShadowDepth='1' Opacity='.45'/></Border.Effect>" +
                "</Border>" +
                "<ControlTemplate.Triggers><Trigger Property='IsMouseOver' Value='True'>" +
                "<Setter TargetName='Dot' Property='Opacity' Value='.78'/>" +
                "</Trigger></ControlTemplate.Triggers>" +
                "</ControlTemplate></Thumb.Template></Thumb></Track.Thumb>" +
                "</Track>" +
                "</Grid>" +
                "<ControlTemplate.Triggers><Trigger Property='IsKeyboardFocusWithin' Value='True'>" +
                "<Setter TargetName='Focus' Property='BorderBrush' Value='" + focus + "'/>" +
                "</Trigger></ControlTemplate.Triggers>" +
                "</ControlTemplate>" +
                "</Setter.Value></Setter></Style>";
            return (Style)XamlReader.Parse(xaml);
        }

        private static Style CreatePrimaryButton()
        {
            Style style = ButtonBaseStyle(typeof(Button), 11);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.PrimaryFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.PrimaryTextKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(1)));
            style.Setters.Add(new Setter(Control.FontWeightProperty, FontWeights.SemiBold));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(12, 6, 12, 6)));

            Trigger hover = Trigger(UIElement.IsMouseOverProperty, true);
            hover.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.PrimaryHoverKey)));
            style.Triggers.Add(hover);
            Trigger pressed = Trigger(ButtonBase.IsPressedProperty, true);
            pressed.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.PrimaryPressedKey)));
            pressed.Setters.Add(new Setter(UIElement.RenderTransformProperty, new TranslateTransform(0, 1)));
            style.Triggers.Add(pressed);
            AddDisabled(style);
            return style;
        }

        private static Style CreateGhostButton()
        {
            Style style = ButtonBaseStyle(typeof(Button), 10);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(1)));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(8, 5, 8, 5)));

            Trigger hover = Trigger(UIElement.IsMouseOverProperty, true);
            hover.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleHoverKey)));
            style.Triggers.Add(hover);
            Trigger pressed = Trigger(ButtonBase.IsPressedProperty, true);
            pressed.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubblePressedKey)));
            pressed.Setters.Add(new Setter(UIElement.RenderTransformProperty, new TranslateTransform(0, 1)));
            style.Triggers.Add(pressed);
            AddDisabled(style);
            return style;
        }

        private static Style CreateToggle()
        {
            Style style = ButtonBaseStyle(typeof(ToggleButton), 10);
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.MutedKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(1)));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(8, 5, 8, 5)));
            style.Setters.Add(new Setter(Control.FontSizeProperty, 12.0));

            Trigger hover = Trigger(UIElement.IsMouseOverProperty, true);
            hover.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleHoverKey)));
            hover.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            style.Triggers.Add(hover);

            Trigger checkedState = Trigger(ToggleButton.IsCheckedProperty, true);
            checkedState.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.SelectedFillKey)));
            checkedState.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            checkedState.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.FocusKey)));
            style.Triggers.Add(checkedState);

            Trigger pressed = Trigger(ButtonBase.IsPressedProperty, true);
            pressed.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubblePressedKey)));
            pressed.Setters.Add(new Setter(UIElement.RenderTransformProperty, new TranslateTransform(0, 1)));
            style.Triggers.Add(pressed);
            AddDisabled(style);
            return style;
        }

        private static Style CreateInput()
        {
            Style style = new Style(typeof(TextBox));
            style.Setters.Add(new Setter(Control.BackgroundProperty, Theme.Dynamic(Theme.BubbleFillKey)));
            style.Setters.Add(new Setter(Control.ForegroundProperty, Theme.Dynamic(Theme.InkKey)));
            style.Setters.Add(new Setter(Control.BorderBrushProperty, Theme.Dynamic(Theme.BubbleEdgeKey)));
            style.Setters.Add(new Setter(Control.BorderThicknessProperty, new Thickness(1)));
            style.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(8, 4, 8, 4)));
            style.Setters.Add(new Setter(Control.VerticalContentAlignmentProperty, VerticalAlignment.Center));
            style.Setters.Add(new Setter(Control.HorizontalContentAlignmentProperty, HorizontalAlignment.Center));
            style.Setters.Add(new Setter(Control.FontWeightProperty, FontWeights.SemiBold));
            style.Setters.Add(new Setter(Control.TemplateProperty, InputTemplate()));
            return style;
        }

        private static Style ButtonBaseStyle(Type type, double radius)
        {
            Style style = new Style(type);
            style.Setters.Add(new Setter(Control.FontFamilyProperty, Theme.UiFont));
            style.Setters.Add(new Setter(Control.FontSizeProperty, 12.5));
            style.Setters.Add(new Setter(Control.CursorProperty, System.Windows.Input.Cursors.Hand));
            style.Setters.Add(new Setter(Control.HorizontalContentAlignmentProperty, HorizontalAlignment.Center));
            style.Setters.Add(new Setter(Control.VerticalContentAlignmentProperty, VerticalAlignment.Center));
            style.Setters.Add(new Setter(UIElement.RenderTransformOriginProperty, new Point(0.5, 0.5)));
            style.Setters.Add(new Setter(UIElement.EffectProperty, Shadow()));
            style.Setters.Add(new Setter(Control.TemplateProperty, ButtonTemplate(type, radius)));
            return style;
        }

        private static ControlTemplate ButtonTemplate(Type type, double radius)
        {
            ControlTemplate template = new ControlTemplate(type);
            FrameworkElementFactory root = new FrameworkElementFactory(typeof(Grid));

            FrameworkElementFactory focus = new FrameworkElementFactory(typeof(Border), "focus");
            focus.SetValue(Border.CornerRadiusProperty, new CornerRadius(radius + 2));
            focus.SetValue(Border.BorderThicknessProperty, new Thickness(2));
            focus.SetValue(Border.BorderBrushProperty, Brushes.Transparent);
            root.AppendChild(focus);

            FrameworkElementFactory body = new FrameworkElementFactory(typeof(Border), "body");
            body.SetValue(Border.MarginProperty, new Thickness(2));
            body.SetValue(Border.CornerRadiusProperty, new CornerRadius(radius));
            body.SetBinding(Border.BackgroundProperty, ParentBinding("Background"));
            body.SetBinding(Border.BorderBrushProperty, ParentBinding("BorderBrush"));
            body.SetBinding(Border.BorderThicknessProperty, ParentBinding("BorderThickness"));
            body.SetBinding(Border.PaddingProperty, ParentBinding("Padding"));

            FrameworkElementFactory layers = new FrameworkElementFactory(typeof(Grid));
            FrameworkElementFactory highlight = new FrameworkElementFactory(typeof(Border));
            highlight.SetValue(Border.HeightProperty, 2.0);
            highlight.SetValue(Border.MarginProperty, new Thickness(4, 1, 4, 0));
            highlight.SetValue(Border.VerticalAlignmentProperty, VerticalAlignment.Top);
            highlight.SetValue(Border.CornerRadiusProperty, new CornerRadius(radius));
            highlight.SetResourceReference(Border.BackgroundProperty, Theme.HighlightKey);
            highlight.SetValue(UIElement.IsHitTestVisibleProperty, false);
            layers.AppendChild(highlight);

            FrameworkElementFactory content = new FrameworkElementFactory(typeof(ContentPresenter));
            content.SetValue(ContentPresenter.RecognizesAccessKeyProperty, true);
            content.SetBinding(ContentPresenter.ContentProperty, ParentBinding("Content"));
            content.SetBinding(ContentPresenter.ContentTemplateProperty, ParentBinding("ContentTemplate"));
            content.SetBinding(ContentPresenter.HorizontalAlignmentProperty, ParentBinding("HorizontalContentAlignment"));
            content.SetBinding(ContentPresenter.VerticalAlignmentProperty, ParentBinding("VerticalContentAlignment"));
            layers.AppendChild(content);
            body.AppendChild(layers);
            root.AppendChild(body);

            if (type == typeof(ToggleButton))
            {
                FrameworkElementFactory indicator =
                    new FrameworkElementFactory(typeof(Border), "selectedIndicator");
                indicator.SetValue(FrameworkElement.WidthProperty, 6.0);
                indicator.SetValue(FrameworkElement.HeightProperty, 6.0);
                indicator.SetValue(FrameworkElement.HorizontalAlignmentProperty, HorizontalAlignment.Right);
                indicator.SetValue(FrameworkElement.VerticalAlignmentProperty, VerticalAlignment.Top);
                indicator.SetValue(FrameworkElement.MarginProperty, new Thickness(0, 6, 7, 0));
                indicator.SetValue(Border.CornerRadiusProperty, new CornerRadius(3));
                indicator.SetValue(UIElement.VisibilityProperty, Visibility.Collapsed);
                indicator.SetValue(UIElement.IsHitTestVisibleProperty, false);
                indicator.SetResourceReference(Border.BackgroundProperty, Theme.FocusKey);
                root.AppendChild(indicator);
            }
            template.VisualTree = root;

            Trigger focused = Trigger(UIElement.IsKeyboardFocusedProperty, true);
            focused.Setters.Add(new Setter(Border.BorderBrushProperty, Theme.Dynamic(Theme.FocusKey), "focus"));
            template.Triggers.Add(focused);
            if (type == typeof(ToggleButton))
            {
                Trigger selected = Trigger(ToggleButton.IsCheckedProperty, true);
                selected.Setters.Add(
                    new Setter(UIElement.VisibilityProperty, Visibility.Visible, "selectedIndicator"));
                template.Triggers.Add(selected);
            }
            return template;
        }

        private static ControlTemplate InputTemplate()
        {
            ControlTemplate template = new ControlTemplate(typeof(TextBox));
            FrameworkElementFactory root = new FrameworkElementFactory(typeof(Grid));
            FrameworkElementFactory focus = new FrameworkElementFactory(typeof(Border), "focus");
            focus.SetValue(Border.CornerRadiusProperty, new CornerRadius(8));
            focus.SetValue(Border.BorderThicknessProperty, new Thickness(2));
            focus.SetValue(Border.BorderBrushProperty, Brushes.Transparent);
            root.AppendChild(focus);

            FrameworkElementFactory body = new FrameworkElementFactory(typeof(Border));
            body.SetValue(Border.MarginProperty, new Thickness(2));
            body.SetValue(Border.CornerRadiusProperty, new CornerRadius(6));
            body.SetBinding(Border.BackgroundProperty, ParentBinding("Background"));
            body.SetBinding(Border.BorderBrushProperty, ParentBinding("BorderBrush"));
            body.SetBinding(Border.BorderThicknessProperty, ParentBinding("BorderThickness"));
            FrameworkElementFactory host = new FrameworkElementFactory(typeof(ScrollViewer), "PART_ContentHost");
            host.SetBinding(Control.PaddingProperty, ParentBinding("Padding"));
            body.AppendChild(host);
            root.AppendChild(body);
            template.VisualTree = root;

            Trigger focused = Trigger(UIElement.IsKeyboardFocusedProperty, true);
            focused.Setters.Add(new Setter(Border.BorderBrushProperty, Theme.Dynamic(Theme.FocusKey), "focus"));
            template.Triggers.Add(focused);
            return template;
        }

        private static Binding ParentBinding(string path)
        {
            Binding binding = new Binding(path);
            binding.RelativeSource = new RelativeSource(RelativeSourceMode.TemplatedParent);
            return binding;
        }

        private static Trigger Trigger(DependencyProperty property, object value)
        {
            Trigger trigger = new Trigger();
            trigger.Property = property;
            trigger.Value = value;
            return trigger;
        }

        private static DropShadowEffect Shadow()
        {
            DropShadowEffect shadow = new DropShadowEffect();
            shadow.BlurRadius = 7;
            shadow.ShadowDepth = 2;
            shadow.Opacity = 0.32;
            shadow.Color = Colors.Black;
            shadow.Freeze();
            return shadow;
        }

        private static void AddDisabled(Style style)
        {
            Trigger disabled = Trigger(UIElement.IsEnabledProperty, false);
            disabled.Setters.Add(new Setter(UIElement.OpacityProperty, 0.38));
            style.Triggers.Add(disabled);
        }
    }
}
