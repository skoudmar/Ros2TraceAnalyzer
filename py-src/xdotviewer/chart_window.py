import os
import os.path
import pathlib

import gi
from gi.repository import GdkPixbuf, GLib, Gtk
from r2ta_interface import R2TAInterface
from ros_element import (
    ChartRequest,
    ChartType,
    ChartValue,
    ElementReference,
    ElementType,
    NodeType,
)

gi.require_version("Gtk", "3.0")


def add_button(toolbar: Gtk.Toolbar, size: int, label: str, icon_name: str, hook):
    pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_size(
        os.path.join(pathlib.Path(__file__).parent.resolve(), "media", icon_name),
        size,
        size,
    )
    icon = Gtk.Image.new_from_pixbuf(pixbuf)
    btn = Gtk.ToolButton(label=label, icon_widget=icon)
    btn.connect("clicked", hook)
    toolbar.insert(btn, -1)


class ChartWindow(Gtk.Window):
    def __init__(self, r2ta: R2TAInterface, element: ElementReference):
        super().__init__()

        self.r2ta = r2ta
        self.element = element
        self.title = True
        self.connect("size-allocate", self.on_reconfigure)
        self.size = (800, 600)
        self.set_default_size(800, 600)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.add(vbox)

        toolbar = Gtk.Toolbar()
        toolbar.set_style(Gtk.ToolbarStyle.ICONS)

        add_button(
            toolbar,
            24,
            "Include title",
            "histogram.svg",
            lambda w: self.set_and_rerun("title", not self.title),
        )
        add_button(
            toolbar,
            24,
            "Scatter plot",
            "scatter.svg",
            lambda w: self.set_and_rerun("chart", ChartType.SCATTER),
        )
        add_button(
            toolbar,
            24,
            "Histogram",
            "histogram.svg",
            lambda w: self.set_and_rerun("chart", ChartType.HISTOGRAM),
        )

        histogram_bins = Gtk.SpinButton(
            adjustment=Gtk.Adjustment(
                value=0,
                lower=0,
                upper=2000,
                step_increment=1,
                page_increment=10,
            ),
            climb_rate=0,
            digits=0,
        )
        histogram_bins.set_placeholder_text("Bin count")
        histogram_bins.set_width_chars(9)
        histogram_bins.connect(
            "value-changed",
            lambda w: self.set_and_rerun("bins", w.get_value_as_int()),
        )

        toolbar.insert(Gtk.ToolItem(child=histogram_bins), -1)

        if self.element.element_type == ElementType.NODE:
            if self.element.node_type == NodeType.CALLBACK:
                self.value = ChartValue.CALLBACK_DURATION
                callback_duration = Gtk.ToolButton(label="Execution duration")
                callback_duration.connect(
                    "clicked",
                    lambda w: self.set_and_rerun("value", ChartValue.CALLBACK_DURATION),
                )
                toolbar.insert(callback_duration, -1)

                activation_delay = Gtk.ToolButton(label="Activation delay")
                activation_delay.connect(
                    "clicked",
                    lambda w: self.set_and_rerun("value", ChartValue.ACTIVATIONS_DELAY),
                )
                toolbar.insert(activation_delay, -1)
            elif self.element.node_type == NodeType.TIMER:
                self.value = ChartValue.ACTIVATIONS_DELAY
            elif self.element.node_type == NodeType.PUBLISHER:
                self.value = ChartValue.PUBLICATIONS_DELAY
            elif self.element.node_type == NodeType.SUBSCRIBER:
                self.value = ChartValue.MESSAGES_DELAY

        elif self.element.element_type == ElementType.EDGE:
            self.value = ChartValue.MESSAGE_LATENCY

        save_as_icon = Gtk.Image.new_from_icon_name("document-save-as", 16)
        save_as = Gtk.ToolButton(label="Save as", icon_widget=save_as_icon)
        save_as.connect("clicked", self.save_as)
        toolbar.insert(save_as, -1)

        export_as_icon = Gtk.Image.new_from_icon_name("document-send", 16)
        export_as = Gtk.ToolButton(label="Export data", icon_widget=export_as_icon)
        export_as.connect("clicked", self.export_as)
        toolbar.insert(export_as, -1)

        self.chart = ChartType.HISTOGRAM
        self.bins = None

        vbox.pack_start(toolbar, False, False, 0)

        image_buffer = self.render()
        self.image = Gtk.Image.new_from_pixbuf(image_buffer)
        self.image.set_size_request(1, 1)

        self.scrolled = Gtk.ScrolledWindow()
        self.scrolled.set_hexpand(True)
        self.scrolled.set_vexpand(True)
        self.scrolled.add(self.image)

        vbox.pack_start(self.scrolled, True, True, 0)

    def on_reconfigure(self, widget, event):
        def resize():
            self.resize_timer = None
            self.set_and_rerun(
                "size",
                (
                    self.scrolled.get_allocated_width(),
                    self.scrolled.get_allocated_height(),
                ),
            )
            return False

        if not hasattr(self, "resize_timer"):
            self.resize_timer = None
            resize()
        else:
            if getattr(self, "resize_timer", None):
                GLib.source_remove(self.resize_timer)
            self.resize_timer = GLib.timeout_add(50, resize)
        return False

    def render(self):
        return self.r2ta.render(
            ChartRequest(
                node=self.element.node,
                value=self.value,
                plot=self.chart,
                bins=self.bins,
                title=self.title,
                size=self.size,
            )
        )

    def visualise(self):
        image_buffer = self.render()
        self.image.set_from_pixbuf(image_buffer)

    def set_and_rerun(self, param, value):
        if getattr(self, param) != value:
            setattr(self, param, value)
            self.visualise()

    def export_as(self, w):
        buttons = (
            Gtk.STOCK_CANCEL,
            Gtk.ResponseType.CANCEL,
            Gtk.STOCK_SAVE,
            Gtk.ResponseType.OK,
        )
        chooser = Gtk.FileChooserDialog(
            parent=self,
            title="Export data",
            action=Gtk.FileChooserAction.SAVE,
            buttons=buttons,
        )
        chooser.set_default_response(Gtk.ResponseType.OK)
        chooser.set_current_folder(os.getcwd())

        if chooser.run() == Gtk.ResponseType.OK:
            filename = chooser.get_filename()
            chooser.destroy()
            self.r2ta.export(filename, self.element.node, self.value)
        else:
            chooser.destroy()

    def save_as(self, w):
        default_filter = "PNG image"

        output_formats = {
            "PNG image": "png",
            "SVG image": "svg",
        }
        buttons = (
            Gtk.STOCK_CANCEL,
            Gtk.ResponseType.CANCEL,
            Gtk.STOCK_SAVE,
            Gtk.ResponseType.OK,
        )
        chooser = Gtk.FileChooserDialog(
            parent=self,
            title="Save chart as",
            action=Gtk.FileChooserAction.SAVE,
            buttons=buttons,
        )
        chooser.set_default_response(Gtk.ResponseType.OK)
        chooser.set_current_folder(os.getcwd())

        for name, ext in output_formats.items():
            filter_ = Gtk.FileFilter()
            filter_.set_name(name)
            filter_.add_pattern("*." + ext)
            chooser.add_filter(filter_)
            if name == default_filter:
                chooser.set_filter(filter_)

        if chooser.run() == Gtk.ResponseType.OK:
            filename = chooser.get_filename()
            chooser.destroy()

            self.r2ta.save_as(
                filename,
                ChartRequest(
                    node=self.element.node,
                    value=self.value,
                    plot=self.chart,
                    bins=self.bins,
                    title=self.title,
                    size=self.size,
                ),
            )
        else:
            chooser.destroy()
