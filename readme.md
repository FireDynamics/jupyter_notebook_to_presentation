# **Presentation**

This program is designed to reduce redundancy for scripts, documentation, etc. implemented as Jupyter Notebooks that also require a presentation with the same content. By tagging the cells inside a Notebook and running this program, a presentation is automatically generated as an R Markdown file. 

(Currently only tested with the [xaringan](https://github.com/yihui/xaringan) styling.)

## **Table of content**
 - [Build from source](##Build-from-source)
 - [Usage](##Usage)


## **Build from source**

To build this program from source, a rustup installation is required. An installation guide can be found [here](https://www.rust-lang.org/tools/install).

Use the following commands to clone and build this project:
```
git clone https://github.com/FireDynamics/jupyter_notebook_to_presentation
cd jupyter_notebook_to_presentation
cargo build --release
```

The build program can be found under `./target/release/presentation` relative to the project path.

## **Usage**
### **Notebook:**
First, cells in a Notebook need to be tagged.

#### **Supported Tags:**
| Tag                   | Use                                                                                                                                                                   |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `new-page`            | initialize a new page.                                                                                                                                                |
| `add-to-page`         | Add the user generated input from a cell to the latest page. Code cells are automatically wrapped in a code block.                                                    |
| `add-stream-to-page`  | Add the stream output of a code cell to the lates page.                                                                                                               |
| `add-error-to-page`   | Add the error of a code cell to the latest page. Note: currently only the type and description are supported not the positioning.                                     |
| `inject-to-page[...]` | Injects the content inside `[...]` to the latest page.                                                                                                                |
| `wrap-image[...]`     | Wraps the image paths in a markdown cell around a formatted string inside `[...]`. An explanation can be found in the [test](tests/notebooks/wrap_images.ipynb) file. |
| `class[...]`          | Sets the class of the latest page to the content inside `[...]`                                                                                                       |

All tags besides `class[...]` are executed in order. `class[...]` can be defined out of order and will run before initializing a new page.


### **Command line**
Are the cells correctly tagged, the program can be run. The supported arguments can be seen by running `presentation -h`

```
Create a presentation from passed `.ipynb` notebooks.

USAGE: [OPTIONS] [input]...

OPTIONS:
    -h,  --help             Prints this help information
    -o,  --output <output>  The path where the presentation will be saved.
    -f,  --force            Force override the file if it already exists.
    -v,  --verbose          Enable verbose output.

ARGS:
    <input>...  The source paths of the notebooks or folders.
```

#### **Example:**
We have the following folder structure:
```
main_folder/
    sub_folder/
        01_sub_page.ipynb
        02_sub_page.ipynb
        ignored_file.rmd
    01_main_page.ipynb
    title.rmd
```
The command could look something like this:
```sh
presentation -o presentation.rmd main_folder/title.rmd main_folder
```
Now the presentation will be filled in the following order:
```
main_folder/title.rmd
main_folder/01_main_page.ipynb
main_folder/sub_folder/01_sub_page.ipynb
main_folder/sub_folder/02_sub_page.ipynb
```
Note:
- The `output path` has to be defined, and all arguments have to be set before the definition of the `input paths`. The `input paths` can direct to a file or a directory. 
- If a `input path` directs to a directory only, but all `.ipynb` files are used recursively.
- If a `input path` directs to a file and it is not a `.ipynb` file the content is injected raw to the presentation.