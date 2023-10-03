use std::io::{self, Write};
use std::path::Path;
use std::thread;

use walkdir::WalkDir;

use ignore::WalkBuilder;

fn main() {
  // let mut path = env::args().nth(1).unwrap();
  // let mut parallel = false;
  // let mut simple = false;
  // if path == "parallel" {
  //     path = env::args().nth(2).unwrap();
  //     parallel = true;
  // } else if path == "walkdir" {
  //     path = env::args().nth(2).unwrap();
  //     simple = true;
  // }

  let path = "./crates/ignore";
  let parallel = true;
  let simple = false;

  let (sender, receiver) = crossbeam_channel::bounded::<DirEntry>(50);

  let stdout_thread = thread::spawn(move || {
    let mut stdout = io::BufWriter::new(io::stdout());
    for dent in receiver {
      // println!(
      //   "thread[{:?}]: got dent from rx > {:?}",
      //   thread::current(),
      //   dent
      // );
      write_path(&mut stdout, dent.path_ref());
      // stdout.flush();
    }
    println!("rx done")
  });

  if parallel {
    let walker = WalkBuilder::new(path).threads(8).build_parallel();
    walker.run(|| {
      let tx = sender.clone();
      let v = Box::new(move |result: Result<ignore::DirEntry, _>| {
        use ignore::WalkState::*;

        let dir = result.unwrap();
        // println!(
        //   "thread[{:?}]: parallel send dir from tx > {:?}",
        //   thread::current(),
        //   dir
        // );
        tx.send(DirEntry::Y(dir)).unwrap();
        Continue
      });
      return v;
    });
  } else if simple {
    let walker = WalkDir::new(path);
    for result in walker {
      let dir = result.unwrap();
      println!(
        "thread[{:?}]: simple send dir from tx > {:?}",
        thread::current(),
        dir
      );
      sender.send(DirEntry::X(dir)).unwrap();
    }
  } else {
    let walker = WalkBuilder::new(path).build();
    for result in walker {
      let dir = result.unwrap();
      println!(
        "thread[{:?}]: normal send dir from tx > {:?}",
        thread::current(),
        dir
      );
      sender.send(DirEntry::Y(dir)).unwrap();
    }
  }

  // 必须先drop掉sender， 否者receiver的loop不会结束: (当想结束消费者的loop时，需要保证生产者已经生成完毕)。这里的drop保证了下面的join方法可以完成
  drop(sender);

  stdout_thread.join().unwrap();
}

#[derive(Debug)]
enum DirEntry {
  X(walkdir::DirEntry),
  Y(ignore::DirEntry),
}

impl DirEntry {
  fn path_ref(&self) -> &Path {
    match self {
      DirEntry::X(x) => x.path(),
      DirEntry::Y(y) => y.path(),
    }
  }
}

#[cfg(unix)]
fn write_path<W: Write>(mut wtr: W, path: &Path) {
  use std::os::unix::ffi::OsStrExt;
  wtr.write(path.as_os_str().as_bytes()).unwrap();
  wtr.write(b"\n").unwrap();
}

#[cfg(not(unix))]
fn write_path<W: Write>(mut wtr: W, path: &Path) {
  wtr.write(path.to_string_lossy().as_bytes()).unwrap();
  wtr.write(b"\n").unwrap();
}
