use crate::{ShellState, history::HistHandling, pipeline::RunningPipeline};

pub struct Job {
    number: usize,
    pipeline: RunningPipeline,
    text: String,
    status: String,
}

pub struct JobsList {
    jobs: Vec<Job>,
}

impl JobsList {
    pub fn new() -> Self {
        JobsList { jobs: Vec::new() }
    }
    pub fn add(&mut self, pipeline: RunningPipeline, text: String) {
        self.jobs.push(Job {
            number: self.jobs.len() + 1,
            pipeline,
            text: text,
            status: "Running".to_string(),
        });
        println!(
            "[{}] {}",
            self.jobs.last().expect("Last is never empty").number,
            self.jobs
                .last()
                .expect("Last is never empty")
                .pipeline
                .get_pid()
        )
    }
    pub fn show(&self) -> String {
        let most_recent_job_no = self.jobs.last().and_then(|job| Some(job.number));
        let Some(most_recent_job_no) = most_recent_job_no else {
            return "".to_string();
        };

        let second_recent_job_no = self
            .jobs
            .get(self.jobs.len() - 2)
            .and_then(|job| Some(job.number));

        let mut out = String::new();

        for job in &self.jobs {
            out.push_str(
                &format!(
                    "[{num}]{recent}  {status:width$}{text}\n",
                    num = job.number,
                    recent = format_recent(job.number, most_recent_job_no, second_recent_job_no),
                    status = job.status,
                    text = job.text,
                    width = 24,
                )[..],
            );
        }
        out
    }

    /*pub fn reap_finished(&mut self) {
        for job in &mut self.jobs {
            if job.pipeline.try_wait() {
                print
            }
        }
    }*/
}

fn format_recent(job_no: usize, most_recent: usize, sec_recent: Option<usize>) -> &'static str {
    if job_no == most_recent {
        "+"
    } else if sec_recent == Some(job_no) {
        "-"
    } else {
        " "
    }
}
